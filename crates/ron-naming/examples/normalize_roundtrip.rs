fn main() {
    let input = std::env::args().nth(1).expect("name");
    let out = ron_naming::normalize::normalize_fqdn_ascii(&input).expect("normalize");
    println!("{}", (out.0).0);
}
