use ron_proto::oap::hello::Hello;
use serde_json as json;

fn main() {
    let hello = Hello::default();
    println!("{}", json::to_string_pretty(&hello).unwrap());
}
