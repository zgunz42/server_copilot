use actix_web::{get, middleware, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use std::error::Error;
use std::fmt::{self, Display};
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, env};
use teloxide::{prelude::*, RequestError};
use teloxide::types::Me;

mod chatbot;
mod gitlab;

#[derive(Debug)]
struct MyError {
    message: String,
}

impl Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for MyError {}

impl From<MyError> for RequestError {
    fn from(error: MyError) -> Self {
        RequestError::Api(teloxide::ApiError::BotBlocked)
    }
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv().ok(); // Load the .env file if it exists

    // Retrieve the BOT_TOKEN from the environment variable
    let bot_token = env::var("BOT_TOKEN").expect("BOT_TOKEN not found in the environment");
    // store gitlab user data

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
        let gitlab_users = Arc::new(Mutex::new(HashMap::<String, gitlab::GitlabUser>::new()));

        async move {
            let cloned_map = gitlab_users.clone();

            println!("Bot info: {:?}", u_me.username());
            println!("Starting bot...");

            chatbot::Command::repl(bot , move |msg: Message, bot: Bot, cmd: chatbot::Command| {
                let cloned_map = cloned_map.clone(); // Clone the Arc
                async move {
                    let mut user = gitlab::GitlabUser::new("".to_string());
                    if msg.chat.username().is_some() {
                        
                        let mut map = cloned_map.lock().unwrap();
                        let username = msg.chat.username().unwrap();
                        
                        println!("username: {}", username);
                        map.entry(username.to_string())
                            .or_insert(gitlab::GitlabUser::new("".to_string()));

                        user = map.get_mut(username).unwrap().clone();
                    }

                    // chatbot::answer(bot, msg, cmd, &mut user).await?;
                    respond(());
                }

                respond(())
            })
            .await;
        }
    });

    match HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .service(index)
    })
    .bind(("127.0.0.1", 8077))?
    .run()
    .await
    {
        Ok(_) => {
            println!("Server started");
            Ok(())
        }
        Err(err) => {
            println!("Error: {:?}", err);

            Err(Box::new(err) as Box<dyn Error + Send + Sync>)
        }
    }
}
