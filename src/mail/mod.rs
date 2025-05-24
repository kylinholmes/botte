use async_imap::Client;
use futures::StreamExt;
use log::info;
use mailparse::{MailHeaderMap, parse_mail};
use tokio::net::TcpStream;
use tokio_native_tls::{TlsConnector, native_tls};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::{
    G_TOKIO_RUNTIME,
    config::{self, CONFIG},
};

pub fn run_mail() {
    if let Some(mail) = CONFIG.mail.clone() {
        G_TOKIO_RUNTIME.spawn(async {
            mail_client(mail).await.unwrap();
        });
    }
}

pub async fn mail_client(mail: config::Mail) -> anyhow::Result<()> {
    info!("[mail] enable mail client");
    let tls = TlsConnector::from(native_tls::TlsConnector::builder().build()?);
    let services = mail.imap_service.split(":").collect::<Vec<&str>>();
    let (host, port) = if services.len() == 2 {
        (services[0].to_string(), services[1].parse::<u16>()?)
    } else {
        (mail.imap_service.clone(), 993) // Default IMAP port
    };
    let tcp_stream = TcpStream::connect((host.clone(), port)).await?;
    let tls_stream = tls.connect(&host, tcp_stream).await?;
    info!("[mail] Connected to IMAP server: {}:{}", host, port);

    let client = Client::new(tls_stream.compat());

    let mut session = client
        .login(&mail.email, &mail.passwd)
        .await
        .map_err(|e| e.0)?;
    info!("[mail] Logged in to IMAP server as: {}", mail.email);

    loop {
        // 4. 检查新邮件
        let unseen = session.search("UNSEEN").await?;
        for seq in unseen.iter() {
            let mut fetches = session.fetch(seq.to_string(), "RFC822").await?;
            while let Some(fetch_result) = fetches.next().await {
                let fetch = fetch_result?;
                if let Some(body) = fetch.body() {
                    // 5. 解析邮件
                    let mail = parse_mail(body)?;
                    let subject = mail.headers.get_first_value("Subject").unwrap_or_default();
                    let from = mail.headers.get_first_value("From").unwrap_or_default();
                    let to = mail.headers.get_first_value("To").unwrap_or_default();
                    // 6. 处理邮件
                    info!(
                        "[mail] New email received: Subject: {}, From: {}, To: {}",
                        subject, from, to
                    );
                    let content = mail.get_body().unwrap_or_default();
                    info!("[mail] Content: {}", content);
                }
            }
        }
        // 7. 等待一段时间再轮询
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
