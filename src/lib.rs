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

use chrono::Local;
use rusqlite::{Connection, Error, params, Result};
use sha2::{Sha256, Digest};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub credit: isize,
    pub payments_in: usize,
    pub payments_out: usize,
    pub password: String,
    pub created: String,
    pub permission: i64,
}

impl User {
    pub fn receive_limit(&self) -> usize {
        (((self.payments_out as f64).sqrt() * 2500.0) as isize + 2500 - self.credit) as usize
    }

    pub fn credit_limit(&self) -> isize {
        ((self.payments_in as f64 * 2.0).powf(0.65) * 200.0) as isize
    }

    pub fn send_limit(&self) -> usize {
        (self.credit_limit() + self.credit) as usize
    }

    pub fn payment_limit(&self, payee: &User) -> SimpletsErr {
        let send_limit = self.send_limit();
        let receive_limit = payee.receive_limit();
        if send_limit <= receive_limit {
            SimpletsErr::PaymentSendLimit(send_limit)
        } else { SimpletsErr::PaymentReceiveLimit(receive_limit) }
    }
}

#[derive(Debug, Serialize)]
pub struct Payment {
    pub id: usize,
    pub payer: usize,
    pub payee: usize,
    pub amount: usize,
    pub created: String,
    pub message: String,
}

#[derive(Debug)]
pub enum SimpletsErr {
    Db(Error),
    PaymentLessMin(usize),
    PaymentSidesEq,
    PaymentReceiveLimit(usize),
    PaymentSendLimit(usize),
    MustNotHappen,
}

impl From<Error> for SimpletsErr {
    fn from(e: Error) -> Self {
        SimpletsErr::Db(e)
    }
}

pub struct Domain {
    pub name: String,
    pub description: String,
    pub conn: Connection,
    pub minimal_amount: usize,
}

impl Domain {
    pub fn new(name: &str, description: &str, minimal_amount: usize) -> Self {
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

    pub fn add_user(&self, name: &str, password: &str) -> Result<usize> {
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

    pub fn add_payment(&mut self, payer: User, payee: User, amount: usize, message: &str) -> Result<(), SimpletsErr> {
        let tx = self.conn.transaction()?;
        if amount < self.minimal_amount { return Err(SimpletsErr::PaymentLessMin(self.minimal_amount)); }
        if payer.id == payee.id { return Err(SimpletsErr::PaymentSidesEq); }
        let limit = payer.payment_limit(&payee);
        match limit {
            SimpletsErr::PaymentSendLimit(l) => if amount > l { return Err(limit) },
            SimpletsErr::PaymentReceiveLimit(l) => if amount > l { return Err(limit) },
            _ => return Err(SimpletsErr::MustNotHappen)
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

#[cfg(test)]
mod tests {
    use crate::User;

    #[test]
    fn payment_limit1() {
        let payer = User::new_icp(0, 1999, 1);
        let u2 = User::new_icp(1, 0, 1);
        assert_eq!(payer.payment_limit(&u2), 2099);
    }
    #[test]
    fn payment_limit2() {
        let payer = User::new_icp(0, 2001, 1);
        let u2 = User::new_icp(1, 0, 0);
        assert_eq!(payer.payment_limit(&u2), 2000);
    }
    #[test]
    fn payment_limit3() {
        let payer = User::new_icp(0, 3200, 3);
        let u2 = User::new_icp(1, -100, 2);
        assert_eq!(payer.payment_limit(&u2), 2900);
    }
}

