pub(crate) fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}
