use std::{sync::{mpsc::{self}, Arc, RwLock}, thread, error::Error};
use actix_web::rt;
use teloxide::{prelude::{self, Dispatcher, Bot}, dispatching::DefaultKey};


mod chatbot;
mod gitlab;
mod context;
mod controller;
mod server;
mod errors;

#[tokio::main]
async fn main() {
    println!("Starting App...");
    let (tx, rx) = mpsc::channel::<Dispatcher<Bot, Box<dyn Error + Send + Sync>, DefaultKey>>();


    let ctxt = Arc::new(RwLock::new(context::Context::new()));

    let m_ctx = Arc::clone(&ctxt);
    thread::spawn(move || {
        let bot_future = chatbot::serve(tx, m_ctx);

        rt::System::new().block_on(bot_future)
    });
    
    // start the bot
    println!("Running server...");


    server::warp_server(ctxt).await;

    println!("The server http stop");

    let _ = rx.recv().unwrap().shutdown_token().shutdown();

    println!("System stopped");

}
