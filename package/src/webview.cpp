//
//  webview.cpp
//  webview
//
//  Created by Mr.Panda on 2023/4/26.
//

#include "webview.h"
#include "app.h"

CefMainArgs get_main_args(int argc, char** argv)
{
#ifdef WIN32
    CefMainArgs main_args(::GetModuleHandleW(nullptr));
#elif LINUX
    CefMainArgs main_args(argc, argv);
#endif

    return main_args;
}

void execute_sub_process(int argc, char** argv)
{
    auto main_args = get_main_args(argc, argv);
    CefExecuteProcess(main_args, new IRenderApp, nullptr);
}

App* create_app(AppSettings* settings, CreateAppCallback callback, void* ctx)
{
    assert(settings);
    assert(callback);

    App* app = new App;
    app->ref = new IApp(settings, callback, ctx);
    app->settings = settings;
    return app;
}

Browser* create_browser(App* app,
                        BrowserSettings* settings,
                        BrowserObserver observer,
                        void* ctx)
{
    assert(app);
    assert(settings);

    Browser* browser = new Browser;
    browser->ref = app->ref->CreateBrowser(settings, observer, ctx);
    return browser;
}

int app_run(App* app, int argc, char** argv)
{
    assert(app);

    auto main_args = get_main_args(argc, argv);
    CefExecuteProcess(main_args, app->ref, nullptr);

    CefSettings cef_settings;
    cef_settings.windowless_rendering_enabled = true;
    cef_settings.chrome_runtime = false;
    cef_settings.no_sandbox = true;
    cef_settings.background_color = 0x00ffffff;

    // macos not support the multi threaded message loop.
#ifdef MACOS
    cef_settings.multi_threaded_message_loop = false;
#else
    cef_settings.multi_threaded_message_loop = true;
#endif

    CefString(&cef_settings.locale).FromString("zh-CN");

    auto cache_path = app->settings->cache_path;
    if (cache_path != nullptr)
    {
        CefString(&cef_settings.cache_path).FromString(cache_path);
        CefString(&cef_settings.log_file).FromString(std::string(cache_path) + "/webview.log");
    }

    auto browser_subprocess_path = app->settings->browser_subprocess_path;
    if (browser_subprocess_path != nullptr)
    {
        CefString(&cef_settings.browser_subprocess_path).FromString(browser_subprocess_path);
    }

    assert(&cef_settings);
    if (!CefInitialize(main_args, cef_settings, app->ref, nullptr))
    {
        return -1;
    }

#ifdef MACOS
    CefRunMessageLoop();
#endif
    return 0;
}

void app_exit(App* app)
{
    assert(app);

#ifdef MACOS
    CefQuitMessageLoop();
#endif
    CefShutdown();
    delete app;
}

void browser_exit(Browser* browser)
{
    assert(browser);

    browser->ref->IClose();
    delete browser;
}

void browser_send_mouse_click(Browser* browser, MouseButtons button, bool pressed)
{
    assert(browser);

    browser->ref->OnMouseClick(button, pressed);
}

void browser_send_mouse_click_with_pos(Browser* browser,
                                       MouseButtons button,
                                       bool pressed,
                                       int x,
                                       int y)
{
    assert(browser);

    browser->ref->OnMouseClickWithPosition(button, x, y, pressed);
}

void browser_send_mouse_wheel(Browser* browser, int x, int y)
{
    assert(browser);

    browser->ref->OnMouseWheel(x, y);
}

void browser_send_mouse_move(Browser* browser, int x, int y)
{
    assert(browser);

    browser->ref->OnMouseMove(x, y);
}

void browser_send_keyboard(Browser* browser, int scan_code, bool pressed, Modifiers modifiers)
{
    assert(browser);

    browser->ref->OnKeyboard(scan_code, pressed, modifiers);
}

void browser_send_touch(Browser* browser,
                        int id,
                        int x,
                        int y,
                        TouchEventType type,
                        TouchPointerType pointer_type)
{
    assert(browser);

    // TouchEventType have the same value with cef_touch_event_type_t.
    // Same as TouchPointerType.
    browser->ref->OnTouch(id, x, y, (cef_touch_event_type_t)type, (cef_pointer_type_t)pointer_type);
}

void browser_bridge_call(Browser* browser, char* req, BridgeCallCallback callback, void* ctx)
{
    assert(browser);
    assert(req);
    assert(callback);

    browser->ref->BridgeCall(req, callback, ctx);
}

void browser_set_dev_tools_open_state(Browser* browser, bool is_open)
{
    assert(browser);

    browser->ref->SetDevToolsOpenState(is_open);
}

void browser_resize(Browser* browser, int width, int height)
{
    assert(browser);

    browser->ref->Resize(width, height);
}

const void* browser_get_hwnd(Browser* browser)
{
    assert(browser);

    auto hwnd = browser->ref->GetHWND();
    return (void*)hwnd;
}

void browser_send_ime_composition(Browser* browser, char* input)
{
    assert(browser);
    assert(input);

    browser->ref->OnIMEComposition(std::string(input));
}

void browser_send_ime_set_composition(Browser* browser, char* input, int x, int y)
{
    assert(browser);
    assert(input);

    browser->ref->OnIMESetComposition(std::string(input), x, y);
}