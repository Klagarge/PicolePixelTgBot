use crate::rank_day::RankDay;
use crate::user::User;
use chrono::{DateTime, Utc};
use sqlx::ConnectOptions;
use sqlx::{Connection, Executor, Row, SqliteConnection, Statement};
use std::str::FromStr;
use sqlx::sqlite::SqliteConnectOptions;
use teloxide::types::{ChatId, MessageId};


pub struct Database {
    path_: String,
}

impl Database {
    pub fn new(path: String) -> Database {
        Database {
            path_: path
        }
    }

    pub async fn create_table(&self) {
        let mut conn = SqliteConnectOptions::from_str(self.path_.as_str())
            .expect("Failed to create database")
            .create_if_missing(true)
            .connect()
            .await
            .expect("Failed to open connection");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS User (\
                        id INTEGER CONSTRAINT user_pk PRIMARY KEY AUTOINCREMENT,\
                        chat_id INTEGER(8) NOT NULL CONSTRAINT user_chat_id UNIQUE,\
                        username TEXT NOT NULL,\
                        hour INTEGER(1) NOT NULL DEFAULT 22)"
        ).await.unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS Rank_day (\
                        id INTEGER CONSTRAINT rank_day_pk PRIMARY KEY AUTOINCREMENT,\
                        user_id INTEGER NOT NULL CONSTRAINT User_id_fk REFERENCES User (id),\
                        time INTEGER(8) NOT NULL,\
                        id_msg INTEGER(4) NOT NULL,\
                        rank INTEGER(1))"
        ).await.unwrap();
        conn.close();
    }

    pub async fn add_user(&self, user: User) -> bool {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("SELECT id, username FROM User WHERE chat_id = ?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(user.get_chat_id().0);

        let result = query.fetch_optional(&mut conn).await.unwrap();

        let user_exist;
        match result {
            None => {
                // add user
                user_exist = false;
                let stmt = conn
                    .prepare("INSERT INTO User (chat_id, username, hour) VALUES (?, ?, ?)")
                    .await
                    .unwrap();

                let query = stmt
                    .query()
                    .bind(user.get_chat_id().0)
                    .bind(user.get_username())
                    .bind(user.get_hour());

                query
                    .execute(&mut conn)
                    .await
                    .expect("Error when inserting new user");
            }
            Some(row) => {
                // modify username
                user_exist = true;
                let stmt = conn
                    .prepare("UPDATE User SET username=? WHERE id=?")
                    .await
                    .unwrap();
                let id: i64 = row.try_get("id").unwrap();
                let query = stmt.query().bind(user.get_username()).bind(id);
                query
                    .execute(&mut conn)
                    .await
                    .expect("Error when updating user");
            }
        }

        conn.close();
        user_exist
    }

    pub async fn add_rank_day(&self, rank_day: RankDay) {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("INSERT INTO Rank_day (user_id, time, id_msg, rank) VALUES (?, ?, ?, ?)")
            .await
            .unwrap();

        let user = rank_day.get_user();
        let user_id = self
            .get_user_id_by_chat_id(user.get_chat_id())
            .await
            .expect("404 User not found");

        let query = stmt
            .query()
            .bind(user_id)
            .bind(rank_day.get_time().timestamp())
            .bind(rank_day.get_id_msg().0)
            .bind(rank_day.get_rank());

        query
            .execute(&mut conn)
            .await
            .expect("Error when inserting new rank_day");

        conn.close();
    }

    async fn get_user_id_by_chat_id(&self, id_chat:ChatId) -> Option<i64> {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("SELECT id FROM User WHERE chat_id = ?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(id_chat.0);

        let result = query.fetch_optional(&mut conn).await.unwrap();

        conn.close();

        match result {
            None => { None }
            Some(row) => {
                let id: i64 = row.try_get("id").unwrap();
                Some(id)
            }
        }
    }

    pub async fn get_user_by_chat_id(&self, id_chat: ChatId) -> Option<User> {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("SELECT chat_id, username, hour FROM User WHERE chat_id = ?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(id_chat.0);

        let result = query.fetch_optional(&mut conn).await.unwrap();

        conn.close();

        match result {
            None => { None }
            Some(row) => {
                let chat_id: i64 = row.try_get("chat_id").unwrap();
                let username: String = row.try_get("username").unwrap();
                let hour: u8 = row.try_get("hour").unwrap();
                let user = User::new(ChatId(chat_id), username, Option::from(hour));
                Some(user)
            }
        }
    }

    pub async fn get_time(&self, id_chat: ChatId, id_msg: MessageId) -> Option<DateTime<Utc>> {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("SELECT time
                            FROM Rank_day
                            join User on User.id = Rank_day.user_id
                            WHERE User.chat_id=? AND Rank_day.id_msg=?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(id_chat.0)
            .bind(id_msg.0);

        let result = query.fetch_optional(&mut conn).await.unwrap();

        conn.close();

        match result {
            None => { None }
            Some(row) => {
                let tst: i64 = row.try_get("time").unwrap();
                let time = DateTime::from_timestamp(tst, 0);
                time
            }
        }
    }

    pub async fn update_rank(&self, id_chat: ChatId, id_msg: MessageId, rank: Option<u8>) {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("UPDATE Rank_day
                            SET rank=?
                            FROM User
                            WHERE User.id=Rank_day.user_id AND
                                  User.chat_id=? AND
                                  Rank_day.id_msg=?")
            .await
            .unwrap();

        let query = stmt
            .query()
            .bind(rank)
            .bind(id_chat.0)
            .bind(id_msg.0);

        query.fetch_optional(&mut conn).await.unwrap();

        conn.close();

    }

    pub async fn set_hour(&self, id_chat: ChatId, hour: u8) -> Result<(), &str> {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("UPDATE User
                            SET hour=?
                            WHERE User.chat_id=?")
            .await
            .unwrap();

        let h = hour.clone();

        let query = stmt
            .query()
            .bind(h)
            .bind(id_chat.0);

        let result = query
            .execute(&mut conn)
            .await;

        conn.close();
        match result {
            Ok(_) => { Ok(()) }
            Err(_) => { Err("Error when updating hour") }
        }

    }

    pub async fn get_hours(&self) -> Vec<(ChatId, u8)> {
        let mut conn = SqliteConnection::connect(self.path_.as_str()).await.unwrap();

        let stmt = conn
            .prepare("SELECT chat_id, hour
                            FROM User")
            .await
            .expect("Error when preparing query");

        let query = stmt
            .query();

        let rows = query.fetch_all(&mut conn).await.unwrap();

        conn.close();

        let mut vec = Vec::new();

        for row in rows {
            let chat_id: i64 = row.try_get("chat_id").unwrap();
            let hour: u8 = row.try_get("hour").unwrap();
            vec.push((ChatId(chat_id), hour));

        }
        vec
    }
}