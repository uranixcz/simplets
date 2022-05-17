use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let domname = args.get(1).expect("domain name");
    let username = args.get(2).expect("user name");
    let password = args.get(3).expect("password");
    let dom = simplets::Domain::new(domname, "", 0);
    dom.add_user(username, password).expect("database error");
}