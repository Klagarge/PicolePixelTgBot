FROM rust:1.67-slim

RUN apt update && apt install -y git pkg-config libssl-dev && apt clean

WORKDIR /usr/src/PicolePixelBot

ADD https://api.github.com/repos/Klagarge/PicolePixelTgBot/git/refs/heads/master ../version.json
RUN git clone https://github.com/Klagarge/PicolePixelTgBot.git ./

RUN cargo install --path .

CMD ["PicolePixelBot"]

# Don't forget to have a .env file with the bot token