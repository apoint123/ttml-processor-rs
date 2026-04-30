use std::{
    env,
    fs,
};

use ttml_processor::parse_ttml;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("用法: cargo run <输入文件.ttml> <输出文件.json>");
        eprintln!("示例: cargo run sample.ttml output.json");
        return;
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let xml_content = match fs::read_to_string(input_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ 读取输入文件失败: {e}");
            return;
        }
    };

    let parse_result = parse_ttml(&xml_content);

    match parse_result {
        Ok(ttml_data) => {
            let output_string = serde_json::to_string_pretty(&ttml_data).unwrap();

            match fs::write(output_path, output_string) {
                Ok(()) => println!("✅ 解析成功！数据已写入到: {output_path}"),
                Err(e) => eprintln!("❌ 写入输出文件失败: {e}"),
            }
        }
        Err(e) => eprintln!("❌ 解析 TTML 失败: {e}"),
    }
}
