use dotenv::dotenv;
use rust_bert::pipelines::common::{ModelResource, ModelType};
use rust_bert::pipelines::question_answering::{
    QaInput, QuestionAnsweringConfig, QuestionAnsweringModel,
};
use rust_bert::resources::LocalResource;
use teloxide::dptree::di::Injectable;
use core::panic;
use std::future::Future;
use std::io::Read;
use std::path::PathBuf;
use std::{env, fs, thread};
use std::{
    error::Error,
    sync::{
        mpsc::Sender,
        Arc, Mutex, RwLock,
    },
};
use teloxide::types::Me;
use teloxide::{
    dispatching::{dialogue::InMemStorage, DefaultKey},
    prelude::*,
};

use crate::context;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    ReceiveFullName,
    ReceiveGitlabToken {
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

    //    Set-up Question Answering model
    let config = QuestionAnsweringConfig::new(
        ModelType::Bert,
        ModelResource::Torch(Box::new(LocalResource {
            local_path: PathBuf::from("/Users/mac/StartUp/digireport-rs/rust_model.ot"),
        })),
        LocalResource {
            local_path: PathBuf::from("/Users/mac/StartUp/digireport-rs/config.json"),
        },
        LocalResource {
            local_path: PathBuf::from("/Users/mac/StartUp/digireport-rs/vocab.txt"),
        },
        None, //merges resource only relevant with ModelType::Roberta
        false,
        false,
        None,
    );

    let qa_model = QuestionAnsweringModel::new(config);
    let qa_model_result: QuestionAnsweringModel;
    match qa_model {
        Ok(model) => {
            qa_model_result = model;
        }
        Err(err) => {
            println!("Error: {:?}", err);
            panic!("Error no model");
        }
    }

    let qa_model_safe = Arc::new(Mutex::new(qa_model_result));

    let memory_state = InMemStorage::<State>::new();
    let deps = dptree::deps![memory_state, ctxt, qa_model_safe];

    let mut server_bot = Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .branch(dptree::case![State::Start].endpoint(start))
            .branch(dptree::case![State::ReceiveFullName].endpoint(receive_full_name))
            .branch(dptree::case![State::ReceiveLocation { full_name, age }].endpoint(receive_location))
            .branch(dptree::case![State::General].endpoint(general))
            .branch(dptree::case![State::ReceiveGitlabToken { full_name }].endpoint(gitlab_token)),
    )
    .dependencies(deps)
    .enable_ctrlc_handler()
    .build();

    server_bot.dispatch().await;

    let _ = tx.send(server_bot);
}


async fn start(
    bot: Bot, 
    dialogue: MyDialogue, 
    ctxt: Arc<RwLock<context::Context>>,
    msg: Message
) -> HandlerResult<()> {
    match msg.text() {
        Some(reply) => {
            let question_1 = reply.to_string();
            let user = msg.from();
            println!("here {}", reply);

            // check string contain / or not
            if question_1.starts_with("/") {
                // switch the cmd and arguments
                let mut parts = question_1[1..].splitn(2, ' ');
                let command = parts.next().unwrap_or("").to_lowercase();
                let argument = parts.next().unwrap_or("");
                println!("here {}", command.as_str());
                match command.as_str() {
                    "repo" | "repository" => {
                        let guser =  user.unwrap();

                        let msg_clone = msg.clone();
                        let id: UserId = guser.clone().id;

                        let kkctx1 = ctxt.read();
                        let kkctx = kkctx1.ok().unwrap();
                        let usaaa = kkctx.get_gitlab_user(id);
                        let ctxtp = usaaa.unwrap().clone();
                        let repositories = ctxtp.get_repositories().await;
                        if repositories.is_ok() {
                            let repos = repositories.unwrap();
                                let mut message = String::new();
        
                                for repo in repos {
                                    message.push_str(&format!(
                                        "id: {}\nname: {}\ndescription: {}\nvisibility: {}\n\n",
                                        repo.id,
                                        repo.name,
                                        repo.description.unwrap_or("".to_string()),
                                        repo.visibility
                                    ));
                                }
                                bot.send_message(msg.chat.id, message).await;
                        }else {
                            bot.send_message(msg.chat.id, "sorry, i don't understand").await;
                        }
                       
                         return Ok(());
                    }
                    "add_token" => {
                        // get all repository of user using token
                        let token = argument.to_string();
                        let user = msg.from();
            
                        match user {
                            Some(user) => {
                                ctxt.write().unwrap().register_gitlab_user(user.id, token);
                                bot.send_message(msg.chat.id, "your token has been saved").await?;
                                dialogue.update(State::Start).await?;
                                return Ok(());
                            }
                            None => {
                                bot.send_message(msg.chat.id, "Error: User not found")
                                    .await?;
                            }
                        }
                    }
                    _ => {
                        bot.send_message(msg.chat.id, "sorry, i don't understand")
                            .await?;
                    }
                }

                dialogue.update(State::Start {}).await?;

                return Ok(());
            }
        }
        None => {
            bot.send_message(msg.chat.id, "sorry, i don't understand")
                            .await?;
        }
    }

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
) -> HandlerResult<()> {
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

async fn receive_full_name(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult<()> {
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
) -> HandlerResult<()> {
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
) -> HandlerResult<()> {
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

fn read_file_to_string(file_path: &std::path::PathBuf) -> std::io::Result<String> {
    let mut file = fs::File::open(file_path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;

    buffer = buffer.replace('\n', " ");

    Ok(buffer)
}

async fn general(
    bot: Bot,
    dialogue: MyDialogue,
    wmodel: Arc<Mutex<QuestionAnsweringModel>>,
    msg: Message,
) -> HandlerResult<()> {
    match msg.text() {
        Some(reply) => {
            let question_1 = reply.to_string();

            // load about me txt
            let context_path = "about_me.txt";
            let current_dir = env::current_dir().expect("Failed to get current directory");

            let file_path = current_dir.join(context_path);
            let context_result = read_file_to_string(&file_path);

            match context_result {
                Ok(file_content) => {
                    // check string contain / or not
                    if question_1.starts_with("/") {
                        // switch the cmd and arguments
                        let mut parts = question_1[1..].splitn(2, ' ');
                        let command = parts.next().unwrap_or("").to_lowercase();
                        let argument = parts.next().unwrap_or("");

                        match command.as_str() {
                            "repo" | "repository" => {
                                // get all repository of user using token
                            }
                            _ => {
                                bot.send_message(msg.chat.id, "sorry, i don't understand")
                                    .await?;
                            }
                        }

                        return Ok(());
                    }

                    let qa_input_1 = QaInput {
                        question: question_1,
                        context: file_content,
                    };

                    let answers = wmodel.lock().unwrap().predict(&[qa_input_1], 1, 32).clone();

                    if answers.len() > 0 {
                        let answer = answers[0][0].clone();
                        println!("{}", answer.score);
                        if answer.score > 0.07 {
                            let text_msg = answer.answer.to_string();

                            bot.send_message(msg.chat.id, text_msg).await?;
                        } else {
                            bot.send_message(msg.chat.id, "sorry, i don't understand")
                                .await?;
                        }
                    } else {
                        bot.send_message(msg.chat.id, "sorry, i don't understand")
                            .await?;
                    }
                }
                Err(err) => {
                    bot.send_message(msg.chat.id, "failed to load information")
                        .await?;
                }
            }

            return Ok(());
        }
        None => {
            bot.send_message(msg.chat.id, "kosong").await?;
        }
    }

    Ok(())
}
