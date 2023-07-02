use actix_web::{get, middleware, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use teloxide::types::Me;
use std::{env};
use teloxide::prelude::*;

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok(); // Load the .env file if it exists

    // Retrieve the BOT_TOKEN from the environment variable
    let bot_token = env::var("BOT_TOKEN").expect("BOT_TOKEN not found in the environment");
    let gitlab_access_token = env::var("GITLAB_ACCESS_TOKEN").expect("GITLAB_ACCESS_TOKEN not found in the environment");
    
    pretty_env_logger::init();
    log::info!("Starting digireport bot...");

    let bot = Bot::new(bot_token);
    let bot_info = bot.get_me();
    let mut me: Option<Me> = None;

    match bot_info.send().await {
        Ok(user) => {
            me = Some(user);
            println!("Bot info: {:?}", me);
        }
        Err(err) => println!("Error: {:?}", err),
    }

    if !me.is_some() {
        println!("Bot info is none");
        return Ok(());
    }

    let u_me = me.unwrap();

    println!("Starting server...");
    
    actix_rt::spawn({
        async move {
            println!("Bot info: {:?}", u_me.username());
            println!("Starting bot...");
            teloxide::repl(bot, |bot: Bot, msg: Message| async move {
                bot.send_message(msg.chat.id, "text").await?;
                respond(())
            }).await;
        }
    });

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .service(index)
    })
    .bind(("127.0.0.1", 8077))?
    .run()
    .await
}