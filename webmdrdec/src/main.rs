use std::{net::SocketAddr, path::PathBuf, process::exit};

use tokio::{fs::File, net::TcpListener};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (listen, path) = {
        let mut args = std::env::args();

        let Some(listen) = args.next() else {
            println!("usage: webmdrdec ADDR FILE");
            exit(1);
        };
        let Ok(listen): Result<SocketAddr, _> = listen.parse() else {
            println!("webmdrdec: {listen:?} is not a valid socket address");
            exit(1);
        };

        let Some(path) = args.next() else {
            println!("usage: webmdrdec ADDR FILE");
            exit(1);
        };

        (listen, PathBuf::from(path))
    };

    if let Err(why) = File::open(path).await {
        println!("cannot listen on {listen}: {why}");
        exit(1);
    }
    let mut tcp = match TcpListener::bind(listen).await {
        Ok(x) => x,
        Err(why) => {
            println!("cannot listen on {listen}: {why}");
            exit(1);
        }
    };
}
