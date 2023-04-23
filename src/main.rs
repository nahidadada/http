use std::fs::{OpenOptions, File};
use std::net::{TcpStream, TcpListener};
use std::path::{PathBuf};
use std::thread;
use std::io::{Read, BufRead, Write, BufWriter, BufReader};

fn main() {
    let local_addr = "127.0.0.1:12345";
    let ret = TcpListener::bind(local_addr);
    if ret.is_err() {
        println!("{:?}", ret.unwrap());
        panic!();
    }    

    let listener = ret.unwrap();
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                handle_client(&stream);
            }
            Err(e) => {
                println!("{:?}", e);
                panic!();
            }
        }        
    }
}

fn handle_client(s: &TcpStream) {
    let ret = s.try_clone();
    if ret.is_err() {
        println!("{:?}", ret.unwrap());
        return;
    }

    let tcpstream = ret.unwrap();
    thread::spawn(move || {
        let ret = tcpstream.try_clone();
        if ret.is_err() {
            println!("{:?}", ret.unwrap());
            return;
        }
        let mut tcp_reader = BufReader::new(ret.unwrap());

        let ret = tcpstream.try_clone();
        if ret.is_err() {
            println!("{:?}", ret.unwrap());
            return;
        }
        let mut tcp_writer = BufWriter::new(ret.unwrap());

        let mut line = String::new();
        let ret = get_line(&mut tcp_reader, &mut line);
        if ret.is_err() {
            println!("{:?}", ret.unwrap());
            return;
        }

        let len = ret.unwrap();
        if len == 0 {
            return;
        }    

        let arr: Vec<&str> = line.split_whitespace().collect();
        if arr.len() < 2 {
            println!("http method error: {}", line);
            return;
        }

        let _cmd = arr[0];
        let url = arr[1];

        let mut path = PathBuf::new();
        path.push("htdocs");
        if !url.eq("/") {
            path.push(url);
        }
        if url.ends_with('/') {
            path.push("index.html");
        }
        if !is_file_exist(&path) {
            discard_header(&mut tcp_reader);
            let ret = not_found(&mut tcp_writer);
            if ret.is_err() {
                println!("send not found error :{}", ret.unwrap());
            }
        } else {
            if path.is_dir() {
                path.push("/index.html");
            }

            serve_file(&mut tcp_reader, &mut tcp_writer, &path);
        }
    });
}

fn serve_file(tcp_reader: &mut BufReader<TcpStream>, tcp_writer: &mut BufWriter<TcpStream>, path: &PathBuf) {
    discard_header(tcp_reader);

    let ret = OpenOptions::new().read(true).open(path);
    if ret.is_err() {
        let ret = not_found(tcp_writer);
        if ret.is_err() {
            println!("send not found error {}", ret.unwrap());
        }
    } else {
        if let Ok(_) = headers(tcp_writer, path) {
            cat(tcp_writer, &ret.unwrap());
        } else {
            println!("send header error");
        }
    }
}

fn discard_header(tcp_reader: &mut BufReader<TcpStream>) {
    let mut line = String::new();
    while let Ok(len) = get_line(tcp_reader, &mut line) {
        if len == 0 {
            println!("get line return 0");
            break;
        }
        if line.eq("\n") {
            break;
        }
        if line.eq("\r\n") {
            break;
        }
        line.clear();
    }
}

fn not_found(stream: &mut BufWriter<TcpStream>) -> Result<usize, std::io::Error> {
    let line = "HTTP/1.0 404 NOT FOUND\r\n";
    stream.write(line.as_bytes())?;

    let line = "Server: jdbhttpd/0.1.0\r\n";
    stream.write(line.as_bytes())?;

    let line = "Content-Type: text/html\r\n";
    stream.write(line.as_bytes())?;

    stream.write("\r\n".as_bytes())?;

    let line = "<HTML><TITLE>Not Found</TITLE>\r\n";
    stream.write(line.as_bytes())?;

    let line = "<BODY><P>The server could not fulfill\r\n";
    stream.write(line.as_bytes())?;

    let line = "your request because the resource specified\r\n";
    stream.write(line.as_bytes())?;

    let line = "is unavailable or nonexistent.\r\n";
    stream.write(line.as_bytes())?;

    let line = "</BODY></HTML>\r\n";
    stream.write(line.as_bytes())?;
    stream.flush()?;

    Ok(0)
}

fn headers(stream: &mut BufWriter<TcpStream>, _path: &PathBuf) -> Result<usize, std::io::Error> {
    stream.write("HTTP/1.0 200 OK\r\n".as_bytes())?;
    stream.write("Server: jdbhttpd/0.1.0\r\n".as_bytes())?;
    stream.write("Content-Type: text/html\r\n".as_bytes())?;
    stream.write("\r\n".as_bytes())?;
    stream.flush()?;
    Ok(0)
}

fn cat(writer: &mut BufWriter<TcpStream>, file: &File) {
    let mut buf = vec![0; 1024];
    let mut reader = BufReader::new(file);

    loop {
        let ret = reader.read(&mut buf);
        if ret.is_err() {
            println!("read file error:{}", ret.unwrap());
            break;
        }
        let len = ret.unwrap();
        if len == 0 {
            break;
        }

        let content = &buf[0..len];
        let ret = writer.write(&content);
        if ret.is_err() {
            println!("tcpstream write error {}", ret.unwrap());
            break;
        }
        assert_eq!(len, ret.unwrap());
        buf.fill(0);
    }
    let _ret = writer.flush();
}

fn get_line(tcp_reader: &mut BufReader<TcpStream>, line: &mut String) -> Result<usize, std::io::Error> {
    return tcp_reader.read_line(line);
}

fn is_file_exist(path: &PathBuf) -> bool {
    let ret = OpenOptions::new().read(true).open(path);
    return ret.is_ok();
}