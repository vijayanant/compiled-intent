use intent::llm::{stream_chat_completion, ChatParams};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_streaming_chat_completion_mock_ollama() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
                {\"model\":\"gemma3:12b\",\"message\":{\"role\":\"assistant\",\"content\":\"LanceDB \"},\"done\":false}\n\
                {\"model\":\"gemma3:12b\",\"message\":{\"role\":\"assistant\",\"content\":\"is \"},\"done\":false}\n\
                {\"model\":\"gemma3:12b\",\"message\":{\"role\":\"assistant\",\"content\":\"fast.\"},\"done\":false}\n\
                {\"model\":\"gemma3:12b\",\"done\":true}\n";
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    let client = reqwest::Client::new();
    let endpoint = format!("http://{}/api/chat", addr);
    let mut writer = Vec::new();

    let params = ChatParams {
        endpoint: &endpoint,
        model: "gemma3:12b",
        temperature: 0.0,
        num_ctx: 16384,
        system_prompt: "System prompt",
        user_prompt: "User question",
    };

    let full_text = stream_chat_completion(&client, &params, &mut writer)
        .await
        .unwrap();

    assert_eq!(full_text, "LanceDB is fast.");
    assert_eq!(String::from_utf8(writer).unwrap(), "LanceDB is fast.");
}

#[tokio::test]
async fn test_streaming_chat_completion_unreachable_endpoint() {
    let client = reqwest::Client::new();
    let endpoint = "http://127.0.0.1:1/api/chat";
    let mut writer = Vec::new();

    let params = ChatParams {
        endpoint,
        model: "gemma3:12b",
        temperature: 0.0,
        num_ctx: 16384,
        system_prompt: "System prompt",
        user_prompt: "User question",
    };

    let err = stream_chat_completion(&client, &params, &mut writer)
        .await
        .unwrap_err();

    let err_msg = err.to_string();
    assert!(err_msg.contains("Failed to connect to LLM endpoint"));
    assert!(err_msg.contains("Ensure Ollama"));
}
