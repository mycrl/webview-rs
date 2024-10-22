//
//  render.h
//  webview
//
//  Created by Mr.Panda on 2023/4/26.
//

#ifndef LIBWEBVIEW_RENDER_H
#define LIBWEBVIEW_RENDER_H
#pragma once

#include <optional>

#include "include/cef_app.h"
#include "webview.h"

class IRender : public CefRenderHandler
{
public:
    IRender(BrowserSettings* settings, BrowserObserver observer, void* ctx);
    ~IRender()
    {
        IClose();
    }

    /* CefRenderHandler */

    virtual bool GetScreenInfo(CefRefPtr<CefBrowser> browser, CefScreenInfo& screen_info) override;
    virtual void OnImeCompositionRangeChanged(CefRefPtr<CefBrowser> browser,
                                              const CefRange& selected_range,
                                              const RectList& character_bounds);
    virtual void GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) override;
    virtual void OnPaint(CefRefPtr<CefBrowser> browser,
                         PaintElementType type,
                         const RectList& dirtyRects,
                         const void* buffer,
                         int width,
                         int height) override;

    void SetBrowser(CefRefPtr<CefBrowser> browser);
    void Resize(int width, int height);
    void IClose();

private:
    std::optional<CefRefPtr<CefBrowser>> _browser = std::nullopt;

    bool is_closed = false;
    BrowserSettings* _settings;
    BrowserObserver _observer;
    void* _ctx;
    int _width;
    int _height;

    IMPLEMENT_REFCOUNTING(IRender);
};

#endif  // LIBWEBVIEW_RENDER_H
