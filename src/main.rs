mod user;
use user::User;

mod rank_day;
use crate::rank_day::RankDay;

use core::option;
use async_std::task;
use chrono::{DateTime, Datelike, Month, Utc};
use lazy_static::lazy_static;
use sqlx::{sqlite, Sqlite, SqlitePool};
use std::error::Error;
use std::sync::{Arc, Mutex};
use serde::de::Unexpected::Option;
use serde_json::from_str;
use teloxide::{
    dispatching::dialogue::{
        serializer::{Bincode, Json},
        ErasedStorage, Storage,
    },
    payloads::SendMessageSetters,
    prelude::*,
    types::*,
    utils::command::BotCommands,
    RequestError,
};
use tokio::time::Duration;

lazy_static! {
    static ref LIST_RANK: [&'static str; 6] = ["0", "1", "2", "3", "4", "5"];
}

lazy_static! {
    static ref USER_LIST: Mutex<Vec<User>> = Mutex::new(Vec::new());
}

lazy_static! {
    static ref RANK_DAY_LIST: Mutex<Vec<RankDay>> = Mutex::new(Vec::new());
}

/// These commands are supported:
#[derive(BotCommands)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Start to use this bot")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Send a photo")]
    Photo,
    #[command(description = "Test")]
    Test,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    println!("Let's go");

    // Create the bot
    let bot = Bot::new("6866868058:AAGGrZ8iPjA_00vygPq24f9DlTHOBL7Bjiw");

    bot.set_my_commands(Command::bot_commands())
        .await
        .expect("Failed to set bot commands");

    tokio::spawn(poll_time(bot.clone()));

    let pool = SqlitePool::connect("sqlite:database.sqlite").await;

    // Create the dispatcher
    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn poll_time(bot: Bot) {
    loop {
        // Get all chat id from user list
        let chat_id = {
            let user_list = USER_LIST.lock().unwrap();
            if user_list.len() == 0 {
                None
            } else {
                Some(user_list[0].get_chat_id())
            }
        };

        if let Some(chat_id) = chat_id {
            let user = User::new(chat_id, "test".to_string(), None);
            let time = Utc::now()+chrono::Duration::days(1);
            let msg_id = send_day_rank_message(bot.clone(), chat_id, std::option::Option::from(time), None).await;
            let rank_day = RankDay::new(user, time, msg_id);
            {
                let mut rank_day_list = RANK_DAY_LIST.lock().unwrap();
                rank_day_list.push(rank_day);

                // print all rank day
                for rank_day in rank_day_list.iter() {
                    println!("Chat id: {}", rank_day.get_chat_id());
                    println!("Time: {}", rank_day.get_time());
                    println!("Id msg: {}", rank_day.get_id_msg());
                }
            }

        }
        task::sleep(Duration::from_secs(30)).await;
    }
}

fn get_month(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => panic!("Month not found"),
    }
}

async fn send_day_rank_message(bot: Bot, chat_id: ChatId, utc_time: option::Option<DateTime<Utc>>, id_msg: std::option::Option<MessageId>) -> MessageId {
    // Define utc time from rank day time if None
    let rank_day_list = RANK_DAY_LIST.lock().unwrap();

    let mut time = Utc::now();
        match utc_time {
        Some(utc_time) => {
            time = utc_time;
        },
        None => {
            // Get rank day from list
            for rank_day in rank_day_list.iter() {
                if rank_day.get_chat_id() == chat_id {
                    time = rank_day.get_time();
                }
            }
        }
    };

    // Format message with date
    let day = time.day();
    let weekday = time.weekday();
    let month = get_month(time.month());
    let year = time.year();
    let text_message = format!("How drunk are you {weekday} {day} {month} {year} ?");

    // Create callback keyboard with ranks
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];
    for rank in LIST_RANK.clone().chunks(6) {
        let row = rank
            .iter()
            .map(|&rank| InlineKeyboardButton::callback(rank.to_owned(), rank.to_owned()))
            .collect();
        keyboard.push(row);
    }

    // Send message or edit message
    let msg = match id_msg {
        Some(id_msg) => {
            bot.edit_message_text(chat_id, id_msg, text_message)
                .reply_markup(InlineKeyboardMarkup::new(keyboard.clone()))
                .await
        }
        None => {
            bot.send_message(chat_id, text_message)
                .reply_markup(InlineKeyboardMarkup::new(keyboard.clone()))
                .await
        }
    };
    match msg {
        Ok(message) => message.id,
        Err(e) => {
            eprintln!("Failed to send or edit message : {:?}", e);
            MessageId(0)
        }
    }
}

async fn send_day_message(bot: Bot, chat_id: ChatId, utc_time: DateTime<Utc>, id_msg: MessageId, rank: String) -> MessageId {
    // Format message with date and rank
    let day = utc_time.day();
    let weekday = utc_time.weekday();
    let month = get_month(utc_time.month());
    let year = utc_time.year();
    let text_message =
        format!("{weekday} {day} {month} {year} you put a {rank} on the Picole Pixel");

    // Create callback keyboard
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];
    let list = ["Edit", "Add comment"];
    for choice in list.chunks(2) {
        let row = choice
            .iter()
            .map(|&rank| InlineKeyboardButton::callback(rank.to_owned(), rank.to_owned()))
            .collect();

        keyboard.push(row);
    }

    // Edit message
    let msg = bot
        .edit_message_text(chat_id, id_msg, text_message)
        .reply_markup(InlineKeyboardMarkup::new(keyboard.clone()))
        .await;
    id_msg
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Chat id: {}", msg.chat.id);
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                // Create user and add to user list
                let mut user_list = USER_LIST.lock().unwrap();
                let user = User::new(msg.chat.id, "test".to_string(), None);
                user_list.push(user.clone());
                println!("User list size: {}", user_list.len());

                //tokio::spawn(create_rank_day(bot.clone(), user.clone()));

                let time = Utc::now();
                let msg_id = send_day_rank_message(bot.clone(), msg.chat.id, std::option::Option::from(time), None).await;
                let rank_day = RankDay::new(user, time, msg_id);
                let mut rank_day_list = RANK_DAY_LIST.lock().unwrap();
                rank_day_list.push(rank_day);

                // Send message
                //tokio::spawn(send_day_rank_message(bot.clone(), msg.chat.id, Utc::now(), None));


            }
            Ok(Command::Help) => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Ok(Command::Photo) => {
                bot.send_chat_action(msg.chat.id, ChatAction::UploadPhoto)
                    .await?;
                let img = bot.get_user_profile_photos(msg.from().unwrap().id).await?;
                bot.send_photo(
                    msg.chat.id,
                    InputFile::file_id(img.photos[0][0].clone().file.id),
                )
                .await?;
            }
            Ok(Command::Test) => {
                let text_message = "Ok, i record your Id chat";
                bot.send_message(msg.chat.id, text_message).await?;
            }
            Err(_) => {
                bot.send_message(msg.chat.id, "Command not fount !").await?;
            }
        }
    }

    Ok(())
}

async fn update_rank(bot: Bot, user: User, id_msg: MessageId, rankStr: String) {
    let rank = rankStr.parse::<u8>().unwrap();
    let mut time = Utc::now();

    {
        // get rank day from list
        let mut rank_day_list = RANK_DAY_LIST.lock().unwrap();
        for rank_day in rank_day_list.iter_mut() {
            if rank_day.get_chat_id() == user.get_chat_id() {
                if rank_day.get_id_msg() == id_msg {
                    rank_day.set_rank(rank);
                    time = rank_day.get_time();
                }
            }
        }
    }
    tokio::spawn(send_day_message(bot.clone(), user.get_chat_id(), time, id_msg, rankStr.to_string()));
}

async fn create_rank_day(bot: Bot, user: User) {
    let id = send_day_rank_message(bot.clone(), user.clone().get_chat_id(), std::option::Option::from(Utc::now()), None).await;

    let rank_day = RankDay::new(user, Utc::now(), id);
    {
        let mut rank_day_list = RANK_DAY_LIST.lock().unwrap();
        rank_day_list.push(rank_day);
    }
}



async fn callback_handler(
    bot: Bot,
    cbq: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref rank) = cbq.data {
        bot.answer_callback_query(&cbq.id).await?;

        let chat_id = cbq.message.clone().unwrap().chat.id;

        if let Some(Message { id, chat, .. }) = cbq.message {
            if rank == "Edit" {
                send_day_rank_message(
                    bot.clone(),
                    chat_id,
                    None,
                    std::option::Option::from(id)
                ).await;
            } else if rank == "Add comment" {
                /*
                let mut text_message = "Add comment";
                bot.answer_callback_query(cbq.id).await?;
                bot.edit_message_text(cbq.message.unwrap().chat.id, cbq.message.unwrap().id, text_message).await?;
                */
            } else { // rank
                // Get user from user list and update rank
                let mut user_list = USER_LIST.lock().unwrap();
                for user in user_list.iter_mut() {
                    if user.get_chat_id() == chat.id {
                        tokio::spawn(update_rank(bot.clone(), user.clone(), id, rank.to_string()));
                    }
                }

                //send_day_message(bot.clone(), chat.id, Utc::now(), id, rank.to_string()).await;
            }
            return Ok(());
        } else if let Some(id) = cbq.inline_message_id {
            // bot.edit_message_text_inline(id, text_message).reply_markup(make_keyboard_answer()).await?;
            panic!("Inline mode Should be never reached !!!");
        }
    }

    log::info!(
        "Callback query from {:?} with data {:?}",
        cbq.from,
        cbq.data
    );
    Ok(())
}
