use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let domname = args.get(1).expect("domain name");
    let dom = simplets::Domain::new(domname, "", 0);
    let mut users = dom.get_users().unwrap();
    println!("id\t\tname\t\tmax-send\tmax-receive\tbalance");
    for u in users.iter_mut() {
        if u.name.len() < 8 {
            u.name.push('\t');
        };
        println!("{}\t{} \t{}\t\t{}\t\t{}", u.id, u.name, u.send_limit(), u.receive_limit(), u.credit);
    }
    println!("found {} users", users.len());
}