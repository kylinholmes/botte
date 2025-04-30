

pub trait Bot {
    fn run(&self);
    fn send_message(&self, chat_id: i64, message: &str);
}