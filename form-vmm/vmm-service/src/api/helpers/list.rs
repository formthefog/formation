pub async fn list(
    State(channel): State<Arc<Mutex<VmmApiChannel>>>,
) -> Result<Json<Vec<VmInfo>>, String> {

    let event = VmmEvent::GetList {
        requestor: "test".to_string(),
    };

    request_receive(channel, event).await
}