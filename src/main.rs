mod user;
use user::User;

mod rank_day;
use crate::rank_day::RankDay;
mod db;
use db::*;

use async_std::task;
use chrono::{DateTime, Datelike, Utc};
use lazy_static::lazy_static;
use sqlx::SqlitePool;
use std::convert::From;
use std::error::Error;
use teloxide::{payloads::SendMessageSetters, prelude::*, types::*, utils::command::BotCommands};
use tokio::time::Duration;

lazy_static! {
    static ref LIST_RANK: [&'static str; 6] = ["0", "1", "2", "3", "4", "5"];
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
    #[command(description = "set your hour to receive message (ex: /sethour 22)")]
    SetHour(u8),
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    println!("Let's go");

    // Create the bot
    let bot = Bot::from_env();
    let myDb = Db::new("sqlite:database.sqlite".to_string()).await;

    bot.set_my_commands(Command::bot_commands())
        .await
        .expect("Failed to set bot commands");

    tokio::spawn(poll_time(bot.clone()));

    //let pool = SqlitePool::connect("sqlite:database.sqlite").await;

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
            let user_list = db::USER_LIST.lock().unwrap();
            if user_list.len() == 0 {
                None
            } else {
                Some(user_list[0].get_chat_id())
            }
        };

        if let Some(chat_id) = chat_id {
            let user = User::new(chat_id, "test".to_string(), None);
            let time = Utc::now() + chrono::Duration::days(1);
            let msg_id =
                send_day_rank_message(bot.clone(), chat_id, std::option::Option::from(time), None)
                    .await;
            let rank_day = RankDay::new(user, time, msg_id);
            tokio::spawn(add_rank_day(rank_day));
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

/// This function send a message with a keyboard to choose a rank
///
/// # Arguments
///
/// * `bot` - The bot for sending message
/// * `chat_id` - The chat id for sending message
/// * `utc_time` - The date for the evaluated day (if None, the date is get from the rank day list)
/// * `id_msg` - The message id for edit message (if None, the message is send)
///
/// # Return
/// Return the message id of the message send or edit
async fn send_day_rank_message(
    bot: Bot,
    chat_id: ChatId,
    utc_time: Option<DateTime<Utc>>,
    id_msg: Option<MessageId>,
) -> MessageId {
    // Define utc time from rank day time if None
    let time;
    match utc_time {
        Some(utc_time) => time = utc_time,
        None => time = get_time(chat_id, id_msg.unwrap()).await.unwrap(),
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
            // Edit message
            bot.edit_message_text(chat_id, id_msg, text_message)
                .reply_markup(InlineKeyboardMarkup::new(keyboard.clone()))
                .await
        }
        None => {
            // Send message
            bot.send_message(chat_id, text_message)
                .reply_markup(InlineKeyboardMarkup::new(keyboard.clone()))
                .await
        }
    };

    // Return message id or 0 if error
    match msg {
        Ok(message) => message.id,
        Err(e) => {
            eprintln!("Failed to send or edit message : {:?}", e);
            MessageId(0)
        }
    }
}

/// This function send a message with the rank for the evaluated day
///
/// # Arguments
/// * `bot` - The bot for sending message
/// * `chat_id` - The chat id for sending message
/// * `utc_time` - The date for the evaluated day
/// * `id_msg` - The message id for edit message
/// * `rank` - The rank for the evaluated day
///
/// # Return
/// Return the message id of the message send or edit
async fn send_day_message(
    bot: Bot,
    chat_id: ChatId,
    utc_time: DateTime<Utc>,
    id_msg: MessageId,
    rank: String,
) -> MessageId {
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
    msg.unwrap().id
}

/// Handler for message
///
/// # Arguments
/// * `bot` - The bot
/// * `msg` - The message received
/// * `me` - The bot information
///
/// # Return
/// Return Ok if no error
async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            // Handle the command `/start`
            Ok(Command::Start) => {
                // Create user and add to user list
                let username = msg.chat.username().expect("Username not found");
                println!("Chat id: {} is with {}", msg.chat.id, username);
                let user = User::new(msg.chat.id, username.to_string(), None);
                tokio::spawn(add_user(user.clone()));

                // Create rank day and add to rank day list
                let time = Utc::now();
                let msg_id = send_day_rank_message(
                    bot.clone(),
                    msg.chat.id,
                    std::option::Option::from(time),
                    None,
                )
                .await;
                let rank_day = RankDay::new(user, time, msg_id);
                tokio::spawn(add_rank_day(rank_day));
                // TODO: Send welcome message
            }

            // Handle the command `/help`
            Ok(Command::Help) => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }

            // Handle the command `/sethour`
            Ok(Command::SetHour(hour)) => {
                tokio::spawn(set_hour(msg.chat.id, hour));
            }

            Err(_) => {
                bot.send_message(msg.chat.id, "Command not fount !").await?;
            }
        }
    }

    Ok(())
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
                /********
                 * EDIT *
                 ********/

                // If edit, send message with rank day list
                tokio::spawn(send_day_rank_message(
                    bot.clone(),
                    chat.id,
                    None,
                    std::option::Option::from(id),
                ));

                // Clear rank in rank day list
                tokio::spawn(clear_rank_in_rank_day_list(chat.id, id));
            } else if rank == "Add comment" {
                /***********
                 * COMMENT *
                 ***********/

                // TODO: Add comment
            } else {
                /********
                 * RANK *
                 ********/

                // Update rank in rank day list
                tokio::spawn(update_rank_in_rank_day_list(
                    chat.id,
                    id,
                    rank.parse::<u8>().unwrap(),
                ));

                // Send message with rank
                let time = get_time(chat_id, id).await.unwrap();
                tokio::spawn(send_day_message(
                    bot.clone(),
                    chat.id,
                    time,
                    id,
                    rank.to_string(),
                ));
            }
            return Ok(());
        }
    }

    log::info!(
        "Callback query from {:?} with data {:?}",
        cbq.from,
        cbq.data
    );
    Ok(())
}
