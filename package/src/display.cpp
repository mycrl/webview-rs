

#include "display.h"

#ifdef OS_WIN
#include <windows.h>
#endif

#include "include/wrapper/cef_helpers.h"

IDisplay::IDisplay(BrowserSettings* settings, BrowserObserver observer, void* ctx)
    : _settings(settings)
    , _observer(observer)
    , _ctx(ctx)
{
    assert(settings);
}

void IDisplay::OnTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title)
{
    CEF_REQUIRE_UI_THREAD();
    if (_is_closed)
    {
        return;
    }

    if (_settings->is_offscreen)
    {
        _observer.on_title_change(title.ToString().c_str(), _ctx);
    }
    else
    {
    #ifdef WIN32
        SetWindowText(browser->GetHost()->GetWindowHandle(), std::wstring(title).c_str());
    #endif
    }
};

void IDisplay::OnFullscreenModeChange(CefRefPtr<CefBrowser> browser, bool fullscreen)
{
    CEF_REQUIRE_UI_THREAD();
    if (_is_closed)
    {
        return;
    }

    if (_settings->is_offscreen)
    {
        _observer.on_fullscreen_change(fullscreen, _ctx);
    }
    else
    {
    #ifdef WIN32
        if (fullscreen)
        {
            SetWindowLong(browser->GetHost()->GetWindowHandle(), GWL_STYLE, WS_VISIBLE | WS_POPUP);
            SetWindowPos(browser->GetHost()->GetWindowHandle(), NULL, 0, 0, GetSystemMetrics(SM_CXSCREEN),
                         GetSystemMetrics(SM_CYSCREEN), SWP_FRAMECHANGED);
        }
        else
        {
            SetWindowLong(browser->GetHost()->GetWindowHandle(), GWL_STYLE,
                          WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_CLIPSIBLINGS | WS_VISIBLE);
            SetWindowPos(browser->GetHost()->GetWindowHandle(), NULL, 0, 0, 1280, 720, SWP_FRAMECHANGED);
        }
    #endif
    }
};

void IDisplay::IClose()
{
    _is_closed = true;
}