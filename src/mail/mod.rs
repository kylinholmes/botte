use async_imap::Client;
use futures::StreamExt;
use log::{info, warn};
use mailparse::{MailHeaderMap, parse_mail};
use tokio::net::TcpStream;
use tokio_native_tls::{TlsConnector, native_tls};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::{
    boardcast::BROADCAST_SENDER, bot::BOTS_TX, config::{self, CONFIG}, G_TOKIO_RUNTIME
};

pub fn run_mail() {
    if let Some(mail) = CONFIG.mail.clone() {
        info!("[mail] enable mail client");
        G_TOKIO_RUNTIME.spawn(async move {
            let m = mail.clone();
            loop {
                let ret = mail_client(m.clone()).await;
                if let Err(e) = ret {
                    warn!("[mail] in mail client: {}", e);
                }
            }
        });
    }
}

pub async fn mail_client(mail: config::Mail) -> anyhow::Result<()> {
    let filer_users = mail.filter_users.clone();
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
        session.select("INBOX").await?;

        let mut to_mark_as_read = Vec::new();

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
                    let content = extract_body(&mail);
                    info!("[mail] Content: {}", content);

                    let from_address = from.split('<').last().and_then(|s| s.split('>').next()).unwrap_or_default().trim();
                    if filer_users.contains(&from_address.to_string()) {
                        info!("[mail] Sub: {} mark as seen", subject);
                        to_mark_as_read.push(seq.to_string());
                        if let Some(tx) = BROADCAST_SENDER.get() {
                            let ret = tx.send(content).await;
                            if let Err(e) = ret {
                                error!("[mail] Failed to send broadcast message: {}", e);
                            } else {
                                info!("[mail] Broadcast message sent successfully");
                            }
                        } else {
                            error!("[mail] BROADCAST_SENDER not initialized");
                        }
                    }
                }
            }
        }
        // 在循环外标记为已读
        for seq in to_mark_as_read {
            let _ = session.store(seq, "+FLAGS (\\Seen)").await?;
        }
        // 7. 等待一段时间再轮询
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

/// 提取邮件正文的改进逻辑
fn extract_body(parsed_mail: &mailparse::ParsedMail) -> String {
    if parsed_mail.subparts.is_empty() {
        // 如果没有子部分，直接返回正文
        parsed_mail.get_body().unwrap_or_default()
    } else {
        // 遍历子部分，查找 text/plain 或 text/html
        for subpart in &parsed_mail.subparts {
            if let Some(content_type) = subpart.headers.get_first_value("Content-Type") {
                if content_type.contains("text/plain") || content_type.contains("text/html") {
                    return subpart.get_body().unwrap_or_default();
                }
            }
        }
        // 如果没有找到合适的子部分，返回空字符串
        String::new()
    }
}
