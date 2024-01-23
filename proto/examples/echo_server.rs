use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use csync_proto::auth::Auth;
use csync_proto::conn::Connection;
use csync_proto::frame::{self, AcceptFrame, ClipboardFrame, DataFrame, Frame};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:7703";

    let args: Vec<String> = env::args().collect();
    let auth = if args.len() >= 2 {
        let password = args[1].as_str();
        Some(Auth::new(password))
    } else {
        None
    };
    let auth = Arc::new(auth);

    let bind: SocketAddr = addr.parse().unwrap();
    let listener = TcpListener::bind(&bind).await.unwrap();

    println!("Server listen on '{}'", addr);
    let (socket, addr) = listener.accept().await.unwrap();
    println!("Accepted connection from {}", addr);

    let mut conn = Connection::new(socket, Arc::clone(&auth));

    let register_frame = conn.must_read_frame().await.unwrap();
    let register = register_frame.expect_register().unwrap();
    println!(
        "The client name is: {}, subs is: {:?}",
        register.publish.unwrap(),
        register.subs
    );

    let auth_frame = auth
        .as_ref()
        .as_ref()
        .map(|auth| auth.build_frame().unwrap());
    conn.write_frame(&Frame::Accept(AcceptFrame {
        version: frame::PROTOCOL_VERSION,
        auth: auth_frame,
    }))
    .await
    .unwrap();

    let mut text = String::new();
    loop {
        let frame = conn.must_read_frame().await.unwrap();
        match frame {
            Frame::Pull => {
                println!("Recv pull request, send text '{}' to client", text);
                conn.write_frame(&Frame::Push(DataFrame {
                    from: None,
                    digest: String::new(),
                    data: ClipboardFrame::Text(text.clone()),
                }))
                .await
                .unwrap();
            }
            Frame::Push(data_frame) => match data_frame.data {
                ClipboardFrame::Text(text_frame) => {
                    println!("Recv push request, save text '{}'", text_frame);
                    text = text_frame;
                    conn.write_frame(&Frame::None).await.unwrap();
                }
                ClipboardFrame::Image(_) => {
                    conn.write_frame(&Frame::Error("unexpect image".to_string()))
                        .await
                        .unwrap();
                }
            },

            _ => conn
                .write_frame(&Frame::Error("unexpect frame type".to_string()))
                .await
                .unwrap(),
        }
    }
}
