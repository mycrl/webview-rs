//
//  browser.h
//  webview
//
//  Created by Mr.Panda on 2023/4/26.
//

#ifndef LIBWEBVIEW_BROWSER_H
#define LIBWEBVIEW_BROWSER_H
#pragma once

#include <optional>

#include "bridge.h"
#include "control.h"
#include "display.h"
#include "include/cef_app.h"
#include "message_router.h"
#include "render.h"
#include "webview.h"

class IBrowser : public CefClient,
    public CefDragHandler,
    public CefContextMenuHandler,
    public CefLoadHandler,
    public CefLifeSpanHandler,
    public IControl,
    public IBridgeMaster,
    public IRender,
    public IDisplay
{
public:
    IBrowser(std::shared_ptr<MessageRouter> router,
             BrowserSettings* settings,
             BrowserObserver observer,
             void* ctx);

    ~IBrowser()
    {
        IClose();
    }

    /* CefContextMenuHandler */

    virtual void OnBeforeContextMenu(CefRefPtr<CefBrowser> browser,
                                     CefRefPtr<CefFrame> frame,
                                     CefRefPtr<CefContextMenuParams> params,
                                     CefRefPtr<CefMenuModel> model) override;
    virtual bool OnContextMenuCommand(CefRefPtr<CefBrowser> browser,
                                      CefRefPtr<CefFrame> frame,
                                      CefRefPtr<CefContextMenuParams> params,
                                      int command_id,
                                      EventFlags event_flags) override;

    /* CefClient */

    virtual CefRefPtr<CefDragHandler> GetDragHandler() override;
    virtual CefRefPtr<CefContextMenuHandler> GetContextMenuHandler() override;
    virtual CefRefPtr<CefDisplayHandler> GetDisplayHandler() override;
    virtual CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() override;
    virtual CefRefPtr<CefLoadHandler> GetLoadHandler() override;
    virtual CefRefPtr<CefRenderHandler> GetRenderHandler() override;
    virtual bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                          CefRefPtr<CefFrame> frame,
                                          CefProcessId source_process,
                                          CefRefPtr<CefProcessMessage> message) override;

    /* CefLoadHandler */

    virtual void OnLoadStart(CefRefPtr<CefBrowser> browser,
                             CefRefPtr<CefFrame> frame,
                             TransitionType transition_type) override;
    virtual void OnLoadEnd(CefRefPtr<CefBrowser> browser,
                           CefRefPtr<CefFrame> frame,
                           int httpStatusCode) override;
    virtual void OnLoadError(CefRefPtr<CefBrowser> browser,
                             CefRefPtr<CefFrame> frame,
                             ErrorCode error_code,
                             const CefString& error_text,
                             const CefString& failed_url) override;

    /* CefLifeSpanHandler */


    virtual void OnAfterCreated(CefRefPtr<CefBrowser> browser) override;
    virtual bool DoClose(CefRefPtr<CefBrowser> browser) override;
    virtual void OnBeforeClose(CefRefPtr<CefBrowser> browser) override;
    virtual bool OnBeforePopup(CefRefPtr<CefBrowser> browser,
                               CefRefPtr<CefFrame> frame,
                               const CefString& target_url,
                               const CefString& target_frame_name,
                               CefLifeSpanHandler::WindowOpenDisposition target_disposition,
                               bool user_gesture,
                               const CefPopupFeatures& popupFeatures,
                               CefWindowInfo& windowInfo,
                               CefRefPtr<CefClient>& client,
                               CefBrowserSettings& settings,
                               CefRefPtr<CefDictionaryValue>& extra_info,
                               bool* no_javascript_access) override;

    /* CefDragHandler */

    virtual bool OnDragEnter(CefRefPtr<CefBrowser> browser,
                             CefRefPtr<CefDragData> dragData,
                             CefDragHandler::DragOperationsMask mask) override;

    void IClose();
    void SetDevToolsOpenState(bool is_open);
    const void* GetHWND();
private:
    std::optional<CefRefPtr<CefBrowser>> _browser = std::nullopt;

    bool _is_closed = false;
    BrowserSettings* _settings;
    BrowserObserver _observer;
    void* _ctx;

    IMPLEMENT_REFCOUNTING(IBrowser);
};

#endif  // LIBWEBVIEW_BROWSER_H
