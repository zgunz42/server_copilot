use dotenv::dotenv;
use rust_bert::bert::{BertConfigResources, BertModelResources, BertVocabResources};
use rust_bert::pipelines::common::{ModelResource, ModelType};
use rust_bert::pipelines::question_answering::{
    QaInput, QuestionAnsweringConfig, QuestionAnsweringModel,
};
use rust_bert::resources::{RemoteResource, LocalResource};
use std::env;
use std::path::PathBuf;
use std::{
    error::Error,
    future::Future,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, RwLock,
    },
    thread, time,
};
use teloxide::types::Me;
use teloxide::{
    dispatching::{dialogue::InMemStorage, DefaultKey},
    dptree::di::Injectable,
    prelude::*,
    utils::command::BotCommands,
    RequestError,
};
use teloxide_macros::BotCommands;

use crate::context;
use crate::gitlab::GitlabUser;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
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

pub async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    user: &mut GitlabUser,
) -> Result<(), RequestError> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Username(username) => {
            bot.send_message(msg.chat.id, format!("Your username is @{username}."))
                .await?
        }
        Command::UsernameAndAge { username, age } => {
            bot.send_message(
                msg.chat.id,
                format!("Your username is @{username} and age is {age}."),
            )
            .await?
        }
        Command::Start => bot.send_message(msg.chat.id, "text").await?,
        Command::InitToken(token) => {
            user.set_token(token);
            bot.send_message(msg.chat.id, "token set").await?
        }
        Command::Repositories => {
            let repositories = user.get_repositories().await;
            let results = match repositories {
                Ok(repositories) => repositories,
                Err(err) => {
                    bot.send_message(msg.chat.id, format!("Error: {:?}", err))
                        .await?;
                    panic!("read message");
                }
            };
            let mut message = String::new();
            for repo in results {
                message.push_str(&format!(
                    "id: {}\nname: {}\ndescription: {}\nvisibility: {}\n\n",
                    repo.id,
                    repo.name,
                    repo.description.unwrap_or("".to_string()),
                    repo.visibility
                ));
            }
            bot.send_message(msg.chat.id, message).await?
        }
    };

    Ok(())
}

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    ReceiveFullName,
    ReceiveGitlabToken {
        full_name: String,
    },
    ReceiveAge {
        full_name: String,
    },
    ReceiveLocation {
        full_name: String,
        age: u8,
    },
    General,
}

pub async fn serve(
    tx: Sender<Dispatcher<Bot, Box<dyn Error + Send + Sync>, DefaultKey>>,
    ctxt: Arc<RwLock<context::Context>>,
) {
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
    }

    let u_me = me.unwrap();

    ctxt.write().unwrap().set_bot(u_me);

    let memory_state = InMemStorage::<State>::new();
    let deps = dptree::deps![memory_state, ctxt];
    let mut server_bot = Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .branch(dptree::case![State::Start].endpoint(start))
            .branch(dptree::case![State::ReceiveFullName].endpoint(receive_full_name))
            .branch(dptree::case![State::ReceiveAge { full_name }].endpoint(receive_age))
            .branch(
                dptree::case![State::ReceiveLocation { full_name, age }].endpoint(receive_location),
            )
            .branch(dptree::case![State::General].endpoint(general))
            .branch(dptree::case![State::ReceiveGitlabToken { full_name }].endpoint(gitlab_token)),
    )
    .dependencies(deps)
    .enable_ctrlc_handler()
    .build();

    server_bot.dispatch().await;

    let _ = tx.send(server_bot);
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Let's start! What's your full name?")
        .await?;
    dialogue.update(State::ReceiveFullName).await?;
    Ok(())
}

async fn gitlab_token(
    bot: Bot,
    dialogue: MyDialogue,
    full_name: String,
    ctxt: Arc<RwLock<context::Context>>,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            bot.send_message(msg.chat.id, "Processing Token").await?;

            let token = text.to_string();
            let user = msg.from();

            match user {
                Some(user) => {
                    ctxt.write().unwrap().register_gitlab_user(user.id, token);
                    bot.send_message(msg.chat.id, "Token saved").await?;
                    dialogue.update(State::General).await?;
                    return Ok(());
                }
                None => {
                    bot.send_message(msg.chat.id, "Error: User not found")
                        .await?;
                }
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Send me token text.").await?;
        }
    }

    Ok(())
}

async fn receive_full_name(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let fullname = text.to_string();
            let text_msg = format!("Give me your gitlab token, {}?", fullname);
            bot.send_message(msg.chat.id, text_msg).await?;
            dialogue
                .update(State::ReceiveGitlabToken {
                    full_name: text.into(),
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }

    Ok(())
}

async fn receive_age(
    bot: Bot,
    dialogue: MyDialogue,
    full_name: String, // Available from `State::ReceiveAge`.
    msg: Message,
) -> HandlerResult {
    match msg.text().map(|text| text.parse::<u8>()) {
        Some(Ok(age)) => {
            bot.send_message(msg.chat.id, "What's your location?")
                .await?;
            dialogue
                .update(State::ReceiveLocation { full_name, age })
                .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Send me a number.").await?;
        }
    }

    Ok(())
}

async fn receive_location(
    bot: Bot,
    dialogue: MyDialogue,
    wmodel: Arc<Mutex<QuestionAnsweringModel>>,
    (full_name, age): (String, u8), // Available from `State::ReceiveLocation`.
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(location) => {
            let report = format!("Full name: {full_name}\nAge: {age}\nLocation: {location}");
            bot.send_message(msg.chat.id, report).await?;
            dialogue.exit().await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }

    Ok(())
}

async fn general(
    bot: Bot,
    dialogue: MyDialogue,
    ctxt: Arc<RwLock<context::Context>>,
    // (full_name, age): (String, u8),
    msg: Message,
) -> HandlerResult {
    // start the

    //    Set-up Question Answering model
    let config = QuestionAnsweringConfig::new(
        ModelType::Bert,
        // ModelResource::Torch(Box::new(RemoteResource::from_pretrained(
        //     BertModelResources::BERT_QA,
        // ))),
        ModelResource::Torch(
          Box::new(
            LocalResource{
                local_path: PathBuf::from("/Users/mac/StartUp/digireport-rs/rust_model.ot"),
            }
          )
        ),
        RemoteResource::from_pretrained(BertConfigResources::BERT_QA),
        RemoteResource::from_pretrained(BertVocabResources::BERT_QA),
        None, //merges resource only relevant with ModelType::Roberta
        false,
        false,
        None,
    );


    let qa_model = QuestionAnsweringModel::new(config);
    let mmodel: QuestionAnsweringModel;
    match qa_model {
        Ok(model) => {
            mmodel = model;
        }
        Err(err) => {
            println!("Error: {:?}", err);
            panic!("Error no model");
        }
    }

    match msg.text() {
        Some(location) => {
            let question_1 = location.to_string();
            let context_1 = String::from("Amy lives in Amsterdam");

            let qa_input_1 = QaInput {
                question: question_1,
                context: context_1,
            };

            let answers = mmodel.predict(&[qa_input_1], 1, 32).clone();

            let text_msg = format!("{:?}", answers);

            bot.send_message(msg.chat.id, text_msg).await?;

            return Ok(());
        }
        None => {
            bot.send_message(msg.chat.id, "kosong").await?;
        }
    }

    Ok(())
}
