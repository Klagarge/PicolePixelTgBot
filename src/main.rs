mod PicolePixelBot;

use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let bot = Bot::new("6866868058:AAGGrZ8iPjA_00vygPq24f9DlTHOBL7Bjiw");

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        bot.send_dice(msg.chat.id).await?;
        Ok(())
    })
        .await;
}