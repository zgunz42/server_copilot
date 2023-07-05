use std::error::Error;
use teloxide::{prelude::*, utils::command::{BotCommands}, RequestError};
use teloxide_macros::BotCommands;

use crate::gitlab::GitlabUser;



#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "handle a username.")]
    Username(String),
    #[command(description = "handle a username and an age.", parse_with = "split")]
    UsernameAndAge { username: String, age: u8 },
    #[command(description = "initialize gitlab user token")]
    InitToken(String),
    #[command(description = "display information")]
    Start,
    #[command(description = "display all repositories")]
    Repositories,
}

pub async fn answer(bot: Bot, msg: Message, cmd: Command, user: &mut GitlabUser) -> Result<(), RequestError> {
    match cmd {
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::Username(username) => {
            bot.send_message(msg.chat.id, format!("Your username is @{username}.")).await?
        }
        Command::UsernameAndAge { username, age } => {
            bot.send_message(msg.chat.id, format!("Your username is @{username} and age is {age}."))
                .await?
        }
        Command::Start => {
            bot.send_message(msg.chat.id, "text").await?
        }
        Command::InitToken(token) => {
            user.set_token(token);
            bot.send_message(msg.chat.id, "token set").await?
        }
        Command::Repositories => {
            let repositories = user.get_repositories().await; 
            let results = match repositories {
                Ok(repositories) => repositories,
                Err(err) => {
                    bot.send_message(msg.chat.id, format!("Error: {:?}", err)).await?;
                    panic!("read message");
                }
            };
            let mut message = String::new();
            for repo in results {
                message.push_str(&format!("id: {}\nname: {}\ndescription: {}\nvisibility: {}\n\n", repo.id, repo.name, repo.description.unwrap_or("".to_string()), repo.visibility));
            }
            bot.send_message(msg.chat.id, message).await?

        }

    };

    Ok(())
}