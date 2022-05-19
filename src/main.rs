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

#[macro_use] extern crate rocket;

//use rocket::tokio::sync::Mutex;
use std::sync::Mutex;
use rocket::serde::Deserialize;
use rocket::{figment, State};
use simplets::Domain;
use rocket::outcome::IntoOutcome;
use rocket::request::{self, FlashMessage, FromRequest, Request};
use rocket::response::{Redirect, Flash};
use rocket::http::{Cookie, CookieJar};
use rocket::form::Form;
use rocket::response::content::RawHtml;
use rocket_dyn_templates::{Template, context};
use rusqlite::Error;

pub type Domains = Mutex<Domain>;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct TemplateDir(bool);

#[derive(FromForm)]
struct Login<'r> {
    username: &'r str,
    password: &'r str
}

#[derive(FromForm)]
struct Password<'r> {
    old: &'r str,
    new: &'r str,
}

#[derive(FromForm)]
struct Payment<'r> {
    payee: i64,
    amount: usize,
    message: &'r str,
}

#[derive(Debug)]
struct User(i64);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<User, Self::Error> {
        request.cookies()
            .get_private("user_id")
            .and_then(|cookie| cookie.value().parse().ok())
            .map(User)
            .or_forward(())
    }
}

#[post("/payment", data = "<payment>")]
fn payment(user: User, domains: &State<Domains>, payment: Form<Payment<'_>>) -> Option<Flash<Redirect>> {
    use simplets::SimpletsErr::*;
    if payment.message.len() > 140 { return Some(Flash::error(Redirect::to(uri!(index)), "Maximální délka zprávy je 140 znaků.")) }
    let mut domain = domains.lock().unwrap();
    let user = domain.get_user(user.0).expect("database error: {}");
    let payee = match domain.get_user(payment.payee) {
        Ok(u) => u,
        Err(Error::QueryReturnedNoRows) => return Some(Flash::error(Redirect::to(uri!(index)), "Příjemce nexistuje")),
        Err(e) => return Some(Flash::error(Redirect::to(uri!(index)), format!("Databázová chyba. Kontaktujte administrátora s podrobnostmi platby<br>{}", e)))
    };
    let flash = match domain.add_payment(user, payee, payment.amount, payment.message) {
        Ok(_) => Flash::success(Redirect::to(uri!(index)), "Platba proběhla úspěšně."),
        Err(Db(e)) => Flash::error(Redirect::to(uri!(index)), format!("Databázová chyba. Kontaktujte administrátora s podrobnostmi platby<br>{}", e)),
        Err(PaymentSidesEq) => Flash::error(Redirect::to(uri!(index)), "Nelze poslat sám sobě"),
        Err(PaymentLessMin(m)) => Flash::error(Redirect::to(uri!(index)), format!("Minimálně lze poslat {} kr.", m)),
        Err(PaymentSendLimit(_)) => Flash::error(Redirect::to(uri!(index)), "Nedostatek prostředků na účtě"),
        Err(PaymentReceiveLimit(l)) => Flash::error(Redirect::to(uri!(index)), format!("Příjemce nemůže přijmout více než {} kr.", l)),
        _ => Flash::error(Redirect::to(uri!(index)), "Neznámá chyba. Kontaktujte administrátora s podrobnostmi platby")
    };
    Some(flash)
}

#[get("/payment")]
fn no_auth_payment() -> Redirect {
    Redirect::to(uri!(login_page))
}

#[get("/")]
fn index(user: User, domains: &State<Domains>, flash: Option<FlashMessage<'_>>) -> Template {
    let domain = domains.lock().unwrap();
    let user = domain.get_user(user.0).expect("database error: {}");
    let payments = domain.get_payments_by_user(user.id).unwrap();
    Template::render("session", context! {
        user: &user,
        receive_limit: user.receive_limit(),
        send_limit: user.send_limit(),
        payments,
        flash: &flash,
    })
}

#[get("/", rank = 2)]
fn no_auth_index() -> Redirect {
    Redirect::to(uri!(login_page))
}

#[get("/login")]
fn login(_user: User) -> Redirect {
    Redirect::to(uri!(index))
}

#[get("/login", rank = 2)]
fn login_page(flash: Option<FlashMessage<'_>>) -> Template {
    Template::render("login", &flash)
}

#[post("/login", data = "<login>")]
fn post_login(jar: &CookieJar<'_>, login: Form<Login<'_>>, domains: &State<Domains>) -> Result<Redirect, Flash<Redirect>> {
    let domain = domains.lock().unwrap();
    let user = if let Ok(u) = domain.get_user_by_name(login.username) { u }
    else { return Err(Flash::error(Redirect::to(uri!(login_page)), "Špatné jméno/heslo.")) };
    drop(domain);
    let hash = simplets::hash(login.password);
    if hash == user.password {
        jar.add_private(Cookie::new("user_id", user.id.to_string()));
        Ok(Redirect::to(uri!(index)))
    } else {
        Err(Flash::error(Redirect::to(uri!(login_page)), "Špatné jméno/heslo."))
    }
}

#[get("/logout")]
fn logout(jar: &CookieJar<'_>) -> Flash<Redirect> {
    jar.remove_private(Cookie::named("user_id"));
    Flash::success(Redirect::to(uri!(login_page)), "Odhlášení proběhlo úspěšně.")
}

#[post("/password", data = "<password>")]
fn password(user: User, domains: &State<Domains>, password: Form<Password<'_>>) -> Option<Flash<Redirect>> {
    let domain = domains.lock().unwrap();
    if simplets::hash(password.old) == domain.get_user(user.0).expect("database error: {}").password {
        if domain.set_password(user.0, password.new).is_ok() {
            Some(Flash::success(Redirect::to(uri!(index)), "Nové heslo nastaveno."))
        } else { Some(Flash::error(Redirect::to(uri!(index)), "Chyba při změně hesla.")) }
    } else { Some(Flash::error(Redirect::to(uri!(index)), "Původní heslo je neplatné.")) }
}

#[get("/password")]
fn password_page(_user: User) -> RawHtml<&'static str> {
    RawHtml(r#"<form action="/password" method="post" accept-charset="utf-8">
         <label for="old">Původní heslo</label><br>
         <input type="password" name="old" id="old" value="" required autofocus /><br>
         <label for="new">Nové heslo</label><br>
         <input type="password" name="new" id="new" value="" required /><br>
         <p><input type="submit" value="Změnit heslo"></p>
      </form>"#)
}

#[get("/password", rank = 2)]
fn no_auth_password() -> Redirect {
    Redirect::to(uri!(login_page))
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let clets = Domain::new("clets", "", 10);

    //let rct = rocket::ignite()
    let rct = rocket::build()
        .attach(Template::fairing())
        .manage(Mutex::new(clets))
        //.mount("/", routes![no_auth_index])
        .mount("/", routes![index, no_auth_index, login, login_page, post_login, logout, payment, no_auth_payment, password, no_auth_password, password_page]);

    let conf: Result<Vec<String>, figment::Error> = rct.figment().extract_inner("template_dir");
    let _result = rct.manage(TemplateDir(if let Ok(dir) = conf {!dir.is_empty()} else {false}))
        .launch().await?;
    Ok(())
}
