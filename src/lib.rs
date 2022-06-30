/*
* Copyright 2022-2022 Michal Mauser
*
* This program is free software: you can redistribute it and/or modify
* it under the terms of the GNU Affero General Public License as published by
* the Free Software Foundation, either version 3 of the License, or
* (at your option) any later version.
*
* This program is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
* GNU Affero General Public License for more details.
*
* You should have received a copy of the GNU Affero General Public License
* along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

#[cfg(test)]
mod tests;

use chrono::Local;
use rusqlite::{Connection, Error, params, Result};
use sha2::{Sha256, Digest};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub credit: i64,
    pub payments_in: u64,
    pub payments_out: u64,
    pub password: String,
    pub created: String,
    pub permission: i64,
}

impl User {
    pub fn receive_limit(&self) -> i64 {
        (((self.payments_out + 1) as f64).sqrt() * 2500.0) as i64 - self.credit
    }

    pub fn credit_limit(&self) -> i64 {
        (((self.payments_in + 1) as f64).sqrt() * 1000.0) as i64 - 1000
    }

    pub fn send_limit(&self) -> i64 {
        self.credit_limit() + self.credit
    }

    pub fn payment_limit(&self, payee: &User) -> Outcome {
        let send_limit = self.send_limit();
        let receive_limit = payee.receive_limit();
        if send_limit <= receive_limit {
            Outcome::PaymentSendLimit(send_limit)
        } else { Outcome::PaymentReceiveLimit(receive_limit) }
    }
}

#[derive(Debug, Serialize)]
pub struct Payment {
    pub id: u64,
    pub payer: u64,
    pub payee: u64,
    pub amount: u64,
    pub created: String,
    pub message: String,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Db(Error),
    PaymentLessMin(u64),
    PaymentSidesEq,
    PaymentReceiveLimit(i64),
    PaymentSendLimit(i64),
    MustNotHappen,
}

impl From<Error> for Outcome {
    fn from(e: Error) -> Self {
        Outcome::Db(e)
    }
}

pub struct Domain {
    pub name: String,
    pub description: String,
    pub conn: Connection,
    pub minimal_amount: u64,
}

impl Domain {
    pub fn new(name: &str, description: &str, minimal_amount: u64) -> Self {
        let conn = Domain::init_database(name);
        Domain {name: name.to_string(), description: description.to_string(), conn, minimal_amount}
    }

    pub fn get_user(&self, id: i64) -> Result<User> {
        self.conn.query_row("SELECT * FROM user WHERE id = ?", [id],
                       |row| {
                           Ok(User {
                               id: row.get(0)?,
                               name: row.get(1)?,
                               credit: row.get(2)?,
                               payments_in: row.get(3)?,
                               payments_out: row.get(4)?,
                               password: row.get(5)?,
                               created: row.get(6)?,
                               permission: row.get(7)?,
                           })
                       })
    }

    pub fn get_user_by_name(&self, name: &str) -> Result<User> {
        self.conn.query_row("SELECT * FROM user WHERE name = ?", [name],
                            |row| {
                                Ok(User {
                                    id: row.get(0)?,
                                    name: row.get(1)?,
                                    credit: row.get(2)?,
                                    payments_in: row.get(3)?,
                                    payments_out: row.get(4)?,
                                    password: row.get(5)?,
                                    created: row.get(6)?,
                                    permission: row.get(7)?,
                                })
                            })
    }

    pub fn get_users(&self) -> Result<Vec<User>> {
        let mut stmt = self.conn.prepare("SELECT * FROM user")?;
        let iter = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                credit: row.get(2)?,
                payments_in: row.get(3)?,
                payments_out: row.get(4)?,
                password: row.get(5)?,
                created: row.get(6)?,
                permission: row.get(7)?,
            })
        })?;
        let mut vec = Vec::new();
        for person in iter {
            match person {
                Ok(u) => vec.push(u),
                Err(e) => return Err(e)
            }
        }
        Ok(vec)
    }

    pub fn add_user(&self, name: &str, password: &str) -> Result<u64> {
        let hash = hash(password);
        let timestamp = Local::now().timestamp();
        self.conn.execute("INSERT INTO user (id, name, credit, payments_in, payments_out, password, created, permission)\
    VALUES (?1, ?2, 0, 0, 0, ?3, datetime('now', 'localtime'), 1)",
                          params![timestamp, name, hash])?;
        Ok(timestamp.try_into().unwrap()) //err will not happen unless someone has bad clock
    }

    pub fn set_password(&self, user_id: i64, new_password: &str) -> Result<usize> {
        let hash = hash(new_password);
        self.conn.execute("UPDATE user SET password = ?1 WHERE id = ?2",
                          params![hash, user_id])
    }

    pub fn get_payments(&self) -> Result<Vec<Payment>> {
        let mut stmt = self.conn.prepare("SELECT * FROM payment")?;
        let iter = stmt.query_map([], |row| {
            Ok(Payment {
                id: row.get(0)?,
                payer: row.get(1)?,
                payee: row.get(2)?,
                amount: row.get(3)?,
                created: row.get(4)?,
                message: row.get(5)?,
            })
        })?;
        let mut vec = Vec::new();
        for person in iter {
            match person {
                Ok(u) => vec.push(u),
                Err(e) => return Err(e)
            }
        }
        Ok(vec)
    }

    pub fn get_payments_by_user(&self, user: i64) -> Result<Vec<Payment>> {
        let mut stmt = self.conn.prepare("SELECT * FROM payment \
        WHERE payer = ?1 OR payee = ?1 ORDER BY created DESC")?;
        let iter = stmt.query_map([&user], |row| {
            Ok(Payment {
                id: row.get(0)?,
                payer: row.get(1)?,
                payee: row.get(2)?,
                amount: row.get(3)?,
                created: row.get(4)?,
                message: row.get(5)?,
            })
        })?;
        let mut vec = Vec::new();
        for person in iter {
            match person {
                Ok(u) => vec.push(u),
                Err(e) => return Err(e)
            }
        }
        Ok(vec)
    }

    pub fn add_payment(&mut self, payer: User, payee: User, amount: u64, message: &str) -> Result<(), Outcome> {
        let tx = self.conn.transaction()?;
        if amount < self.minimal_amount { return Err(Outcome::PaymentLessMin(self.minimal_amount)); }
        if payer.id == payee.id { return Err(Outcome::PaymentSidesEq); }
        let limit = payer.payment_limit(&payee);
        match limit {
            Outcome::PaymentSendLimit(l) => if amount as i64 > l { return Err(limit) },
            Outcome::PaymentReceiveLimit(l) => if amount as i64 > l { return Err(limit) },
            _ => return Err(Outcome::MustNotHappen)
        }
        tx.execute("UPDATE user SET credit = credit - ?1, payments_out = payments_out + 1 WHERE id = ?2", params![amount, payer.id])?;
        tx.execute("UPDATE user SET credit = credit + ?1, payments_in = payments_in + 1 WHERE id = ?2", params![amount, payee.id])?;
        tx.execute("INSERT INTO payment (payer, payee, amount, created, message)\
        VALUES (?1, ?2, ?3, datetime('now', 'localtime'), ?4)", params![&payer.id, &payee.id, &amount, &message])?;
        tx.commit()?;
        Ok(())
    }

    fn init_database(name: &str) -> Connection {
        let path = format!("{}.sqlite", name);
        let conn = Connection::open(&path).expect("db file");
        let db_version: i64 = conn.query_row("PRAGMA user_version",[], |row| {row.get(0)})
            .expect("lookup db table version");
        if db_version == 0 {
            conn.execute("PRAGMA user_version = 1", []).expect("alter db version");
            conn.execute("PRAGMA foreign_keys = ON", []).expect("change pragma");
            conn.execute("CREATE TABLE user (
                    id              INTEGER PRIMARY KEY,
                    name            TEXT,
                    credit          INTEGER NOT NULL,
                    payments_in     INTEGER NOT NULL,
                    payments_out    INTEGER NOT NULL,
                    password        TEXT NOT NULL,
                    created         TEXT NOT NULL,
                    permission      INTEGER NOT NULL
                    )", [])
                .expect("create table");
            conn.execute("CREATE TABLE payment (
                    id              INTEGER PRIMARY KEY,
                    payer           INTEGER NOT NULL,
                    payee           INTEGER NOT NULL,
                    amount          INTEGER NOT NULL,
                    created         TEXT NOT NULL,
                    message         TEXT NOT NULL,
                    FOREIGN KEY(payer) REFERENCES user(id),
                    FOREIGN KEY(payee) REFERENCES user(id)
                    )", [])
                .expect("create table");
        }
        conn
    }
}

pub fn hash(data: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

