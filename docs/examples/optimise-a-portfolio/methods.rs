fn main() {
    for method in finalytics::methods() {
        println!("{}", method.label);
    }
}
