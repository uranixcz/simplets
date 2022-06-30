use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let domname = args.get(1).expect("domain name");
    let dom = simplets::Domain::new(domname, "", 0);
    let users = dom.get_users().unwrap();
    let sum: i64 = users.iter().map(|u| u.credit).sum();
    assert_eq!(sum, 0);
    for u in users.iter() {
        if u.receive_limit() < 0 {
            println!("user {} has sus funds", u.name);
        }
    }
}