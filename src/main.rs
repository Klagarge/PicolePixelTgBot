use std::error::Error;
use std::sync::{Mutex, Arc};
use lazy_static::lazy_static;
use tokio::time::{Duration, Instant};
use async_std::task;
use std::time::SystemTime;
use teloxide::{
    dispatching::dialogue::{
        serializer::{Bincode, Json},
        ErasedStorage, SqliteStorage, Storage, self, GetChatId, InMemStorage
    },
    prelude::*,
    utils::command::BotCommands,
    types::*,
    payloads::SendMessageSetters,
};

lazy_static! {
    static ref LIST_RANK: [&'static str; 6] = ["0", "1", "2", "3", "4", "5"];
}


lazy_static! {
    static ref LAST_CHAT_ID: Arc<Mutex<Option<ChatId>>> = Arc::new(Mutex::new(None));
}


/// These commands are supported:
#[derive(BotCommands)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
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

    bot.set_my_commands(Command::bot_commands()).await.expect("Failed to set bot commands");

    let last_chat_id = Arc::clone(&LAST_CHAT_ID);
    tokio::spawn(poll_time(bot.clone(), last_chat_id));

    // Create the dispatcher
    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        ;

    Dispatcher::builder(bot, handler).enable_ctrlc_handler().build().dispatch().await;
}

async fn poll_time(bot: Bot, last_chat_id: Arc<Mutex<Option<ChatId>>>) {
    loop {
        let chat_id = {
            let mut last_chat_id_guard = last_chat_id.lock().unwrap();
            if let Some(chat_id) = *last_chat_id_guard {
                Some(chat_id)
            } else {
                None
            }
        };

        if let Some(mut chatId) = chat_id {
            bot.send_message(chatId, "Ping").await.expect("TODO: panic message");
        }
        task::sleep(Duration::from_secs(10)).await;
    }
}


async fn send_message(bot: Bot) {
    let last_chat_id = LAST_CHAT_ID.lock().unwrap();
    if let Some(mut chatId) = *last_chat_id {
        chatId.0 = 241592643;
        bot.send_message(chatId, "Ping").await.expect("TODO: panic message");
    }
    println!("Hey, i send a message");
}

/// Create a keyboard made by buttons
fn make_keyboard_rank() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let list_long = [
        "0 - No Drink",
        "1 - Just one glass",
        "2 - More than one, but no Feeling",
        "3 - Feeling but manageable",
        "4 - Definitely drunk",
        "5 - Almost dead",
    ];
    let list_short = ["0", "1", "2", "3", "4", "5"];
    let list = LIST_RANK.clone();

    for rank in list.chunks(6) {
        let row = rank
            .iter()
            .map(|&rank| InlineKeyboardButton::callback(rank.to_owned(), rank.to_owned()))
            .collect();

        keyboard.push(row);
    }

    InlineKeyboardMarkup::new(keyboard)
}

fn make_keyboard_answer() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let list = [
        "Edit",
        "Add comment",
    ];

    for choice in list.chunks(2) {
        let row = choice
            .iter()
            .map(|&rank| InlineKeyboardButton::callback(rank.to_owned(), rank.to_owned()))
            .collect();

        keyboard.push(row);
    }

    InlineKeyboardMarkup::new(keyboard)
}

async fn message_handler(bot: Bot, msg: Message, me: Me) -> Result<(), Box<dyn Error + Send + Sync>> {
    {
        let mut lastChatId = LAST_CHAT_ID.lock().unwrap();
        *lastChatId = Some(msg.chat.id);
    }
    println!("Msg chat id: {}", msg.chat.id);
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Help) => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
            }
            Ok(Command::Photo) => {
                bot.send_chat_action(msg.chat.id, ChatAction::UploadPhoto).await?;
                let img = bot.get_user_profile_photos(msg.from().unwrap().id).await?;

                bot.send_photo(msg.chat.id, InputFile::file_id(img.photos[0][0].clone().file.id)).await?;
            }
            Ok(Command::Test) => {
                let mut text_message = "How drunk are you Monday 01 January 2024 ?";
                bot.send_message(msg.chat.id, text_message).reply_markup(make_keyboard_rank()).await?;

            }
            Err(_) => {
                bot.send_message(msg.chat.id, "Command not fount !").await?;
            }
        }
    }

    Ok(())
}

async fn callback_handler(bot: Bot, cbq: CallbackQuery) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref rank) = cbq.data {
        bot.answer_callback_query(&cbq.id).await?;

        if let Some(Message {id, chat, ..}) = cbq.message {

            if rank == "Edit" {
                let mut text_message = "How drunk are you Monday 01 January 2024 ?";
                bot.edit_message_text(chat.id, id, text_message).reply_markup(make_keyboard_rank()).await?;
                return Ok(());
            } else if rank == "Add comment" {
                /*
                let mut text_message = "Add comment";
                bot.answer_callback_query(cbq.id).await?;
                bot.edit_message_text(cbq.message.unwrap().chat.id, cbq.message.unwrap().id, text_message).await?;
                */
                return Ok(());
            }

            let mut text_message = format!("Monday 01 January 2024 you put a {rank} on the Picole Pixel");

            bot.edit_message_text(chat.id, id, text_message).reply_markup(make_keyboard_answer()).await?;

        } else if let Some(id) = cbq.inline_message_id {

            let mut text_message = format!("Monday 01 January 2024 you put a {rank} on the Picole Pixel");
            bot.edit_message_text_inline(id, text_message).reply_markup(make_keyboard_answer()).await?;

        }
    }

    log::info!("Callback query from {:?} with data {:?}", cbq.from, cbq.data);
    Ok(())
}
