use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Mutex, Once, OnceLock};
use std::time::Duration;

use futures::channel::oneshot;
use gpui::{AsyncApp, Context, WeakEntity};
use serde_json::json;

use crate::AcpThread;

enum BridgeOp {
    ListSessions {
        reply: oneshot::Sender<Vec<String>>,
    },
    SendPrompt {
        session_id: String,
        text: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Interrupt {
        session_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

static THREADS: OnceLock<Mutex<HashMap<String, WeakEntity<AcpThread>>>> = OnceLock::new();
static BRIDGE_TX: OnceLock<async_channel::Sender<BridgeOp>> = OnceLock::new();
static BRIDGE_STARTED: Once = Once::new();

fn threads_map() -> &'static Mutex<HashMap<String, WeakEntity<AcpThread>>> {
    THREADS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn register_thread(
    session_id: String,
    weak: WeakEntity<AcpThread>,
    cx: &mut Context<AcpThread>,
) {
    if session_id.is_empty() {
        return;
    }
    if let Ok(mut map) = threads_map().lock() {
        map.retain(|_, w| w.upgrade().is_some());
        map.insert(session_id, weak);
    }

    #[cfg(not(test))]
    {
        let async_app = cx.to_async();
        BRIDGE_STARTED.call_once(move || {
            start_bridge(async_app);
        });
    }
    #[cfg(test)]
    let _ = cx;
}

#[cfg(not(test))]
fn bridge_state_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".gemini")
            .join("agynotice")
            .join("zed_bridge.json")
    })
}

#[cfg(not(test))]
fn write_bridge_info(port: u16) {
    let Some(path) = bridge_state_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let payload = json!({
        "pid": std::process::id(),
        "port": port,
    });
    let _ = std::fs::write(path, payload.to_string());
}

#[cfg(not(test))]
fn start_bridge(async_app: AsyncApp) {
    let (tx, rx) = async_channel::unbounded::<BridgeOp>();
    let _ = BRIDGE_TX.set(tx);

    // Foreground GPUI task that executes AcpThread operations on the main UI thread.
    let app_for_loop = async_app.clone();
    async_app
        .spawn(async move |cx| {
            while let Ok(op) = rx.recv().await {
                match op {
                    BridgeOp::ListSessions { reply } => {
                        let ids = if let Ok(mut map) = threads_map().lock() {
                            map.retain(|_, w| w.upgrade().is_some());
                            map.keys().cloned().collect()
                        } else {
                            Vec::new()
                        };
                        let _ = reply.send(ids);
                    }
                    BridgeOp::SendPrompt {
                        session_id,
                        text,
                        reply,
                    } => {
                        let weak = threads_map()
                            .lock()
                            .ok()
                            .and_then(|map| map.get(&session_id).cloned());
                        let Some(weak) = weak else {
                            let _ = reply.send(Err("session_not_open".to_string()));
                            continue;
                        };
                        let res = weak.update(cx, |thread, cx| {
                            let submission = thread.send(vec![text.into()], cx);
                            cx.spawn(async move |_, _| {
                                let _ = submission.await;
                            })
                            .detach();
                        });
                        match res {
                            Ok(()) => {
                                let _ = reply.send(Ok(()));
                            }
                            Err(_) => {
                                if let Ok(mut map) = threads_map().lock() {
                                    map.remove(&session_id);
                                }
                                let _ = reply.send(Err("session_closed".to_string()));
                            }
                        }
                    }
                    BridgeOp::Interrupt { session_id, reply } => {
                        let weak = threads_map()
                            .lock()
                            .ok()
                            .and_then(|map| map.get(&session_id).cloned());
                        let Some(weak) = weak else {
                            let _ = reply.send(Err("session_not_open".to_string()));
                            continue;
                        };
                        let res = weak.update(cx, |thread, cx| {
                            thread.cancel(cx).detach();
                        });
                        match res {
                            Ok(()) => {
                                let _ = reply.send(Ok(()));
                            }
                            Err(_) => {
                                if let Ok(mut map) = threads_map().lock() {
                                    map.remove(&session_id);
                                }
                                let _ = reply.send(Err("session_closed".to_string()));
                            }
                        }
                    }
                }
            }
            drop(app_for_loop);
        })
        .detach();

    let _ = std::thread::Builder::new()
        .name("agynotice-zed-bridge".to_string())
        .spawn(move || {
            let Ok(listener) = TcpListener::bind("127.0.0.1:0") else {
                return;
            };
            let Ok(addr) = listener.local_addr() else {
                return;
            };
            write_bridge_info(addr.port());
            log::info!("AgyNotice Zed bridge listening on http://127.0.0.1:{}", addr.port());

            for stream in listener.incoming().flatten() {
                let _ = std::thread::Builder::new()
                    .name("agynotice-zed-req".to_string())
                    .spawn(move || handle_connection(stream));
            }
        });
}

#[cfg(not(test))]
fn handle_connection(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
        return;
    }
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let path = parts[1].split('?').next().unwrap_or("/");

    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse().unwrap_or(0).min(1024 * 1024);
            }
        }
    }

    let mut body_bytes = vec![0u8; content_length];
    if content_length > 0 && reader.read_exact(&mut body_bytes).is_err() {
        send_json(&mut stream, 400, json!({"ok": false, "error": "invalid_body"}));
        return;
    }

    let Some(tx) = BRIDGE_TX.get() else {
        send_json(&mut stream, 503, json!({"ok": false, "error": "bridge_not_ready"}));
        return;
    };

    match (method, path) {
        ("GET", "/sessions") => {
            let (reply_tx, reply_rx) = oneshot::channel();
            if tx.send_blocking(BridgeOp::ListSessions { reply: reply_tx }).is_ok() {
                if let Ok(sessions) = futures::executor::block_on(reply_rx) {
                    send_json(
                        &mut stream,
                        200,
                        json!({"ok": true, "pid": std::process::id(), "sessions": sessions}),
                    );
                    return;
                }
            }
            send_json(&mut stream, 500, json!({"ok": false, "error": "dispatch_failed"}));
        }
        ("POST", "/send") => {
            let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body_bytes) else {
                send_json(&mut stream, 400, json!({"ok": false, "error": "invalid_json"}));
                return;
            };
            let cid = val
                .get("conversationId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let text = val
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if cid.is_empty() || text.is_empty() {
                send_json(&mut stream, 400, json!({"ok": false, "error": "missing_fields"}));
                return;
            }
            let (reply_tx, reply_rx) = oneshot::channel();
            if tx
                .send_blocking(BridgeOp::SendPrompt {
                    session_id: cid,
                    text,
                    reply: reply_tx,
                })
                .is_ok()
            {
                match futures::executor::block_on(reply_rx) {
                    Ok(Ok(())) => {
                        send_json(&mut stream, 200, json!({"ok": true}));
                        return;
                    }
                    Ok(Err(err)) => {
                        send_json(&mut stream, 200, json!({"ok": false, "error": err}));
                        return;
                    }
                    Err(_) => {}
                }
            }
            send_json(&mut stream, 500, json!({"ok": false, "error": "dispatch_failed"}));
        }
        ("POST", "/interrupt") => {
            let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body_bytes) else {
                send_json(&mut stream, 400, json!({"ok": false, "error": "invalid_json"}));
                return;
            };
            let cid = val
                .get("conversationId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if cid.is_empty() {
                send_json(&mut stream, 400, json!({"ok": false, "error": "missing_conversation_id"}));
                return;
            }
            let (reply_tx, reply_rx) = oneshot::channel();
            if tx
                .send_blocking(BridgeOp::Interrupt {
                    session_id: cid,
                    reply: reply_tx,
                })
                .is_ok()
            {
                match futures::executor::block_on(reply_rx) {
                    Ok(Ok(())) => {
                        send_json(&mut stream, 200, json!({"ok": true}));
                        return;
                    }
                    Ok(Err(err)) => {
                        send_json(&mut stream, 200, json!({"ok": false, "error": err}));
                        return;
                    }
                    Err(_) => {}
                }
            }
            send_json(&mut stream, 500, json!({"ok": false, "error": "dispatch_failed"}));
        }
        _ => {
            send_json(&mut stream, 404, json!({"ok": false, "error": "not_found"}));
        }
    }
}

#[cfg(not(test))]
fn send_json(stream: &mut TcpStream, status: u16, body: serde_json::Value) {
    let raw = body.to_string();
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    let resp = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        status_text,
        raw.len(),
        raw
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}
