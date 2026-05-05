use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use serde::Serialize;
use std::io::{self, Write};
use std::process::Command;

#[derive(Parser)]
struct Args {
    address: String,

    #[arg(long)]
    rpc_url: String,
}

#[derive(Debug, Serialize)]
struct SelectorInfo {
    selector: String,
    mutability: Option<String>,
    return_type: Option<String>,
    signatures: Vec<String>,
}

fn cast(args: &[&str]) -> Result<String> {
    let output = Command::new("cast")
        .args(args)
        .output()
        .with_context(|| format!("failed to run cast {:?}", args))?;

    if !output.status.success() {
        anyhow::bail!(
            "cast {:?} failed:\n{}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn ask(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

fn fetch_bytecode(address: &str, rpc_url: &str) -> Result<String> {
    let code = cast(&["code", address, "--rpc-url", rpc_url])?;

    if code.trim() == "0x" {
        anyhow::bail!("address has no bytecode");
    }

    Ok(code.trim().to_string())
}

fn parse_selectors(output: &str) -> Vec<(String, Option<String>, Option<String>)> {
    let selector_re = Regex::new(r"^0x[0-9a-fA-F]{8}$").unwrap();
    let mut results = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        let selector = parts[0].to_lowercase();

        if !selector_re.is_match(&selector) {
            continue;
        }

        let mut return_type = None;
        let mut mutability = None;

        for part in parts.iter().skip(1) {
            match *part {
                "view" | "pure" | "payable" | "nonpayable" => {
                    mutability = Some(part.to_string());
                }
                _ => {
                    return_type = Some(part.to_string());
                }
            }
        }

        results.push((selector, mutability, return_type));
    }

    results.sort_by(|a, b| a.0.cmp(&b.0));
    results.dedup_by(|a, b| a.0 == b.0);

    results
}

fn fetch_signatures(selector: &str) -> Vec<String> {
    let output = cast(&["4byte", selector]).unwrap_or_else(|_| "".to_string());

    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn scan_contract(address: &str, rpc_url: &str) -> Result<Vec<SelectorInfo>> {
    let code = fetch_bytecode(address, rpc_url)?;
    let selectors_raw = cast(&["selectors", &code])?;
    let parsed_selectors = parse_selectors(&selectors_raw);

    let selectors = parsed_selectors
        .into_iter()
        .map(|(selector, mutability, return_type)| {
            let signatures = fetch_signatures(&selector);

            SelectorInfo {
                selector,
                mutability,
                return_type,
                signatures,
            }
        })
        .collect();

    Ok(selectors)
}

fn display_functions(selectors: &[SelectorInfo]) {
    println!("\nDetected functions:\n");

    for (index, info) in selectors.iter().enumerate() {
        let function_name = first_signature(info);
        let mutability = info.mutability.as_deref().unwrap_or("unknown");

        println!(
            "[{}] {} | {} | {}",
            index + 1,
            info.selector,
            function_name,
            mutability
        );
    }
}

fn choose_function(selectors: &[SelectorInfo]) -> Result<&SelectorInfo> {
    let input = ask("\nChoose function index: ")?;

    let index: usize = input
        .parse()
        .context("invalid index, please enter a number")?;

    if index == 0 || index > selectors.len() {
        anyhow::bail!("index out of range");
    }

    Ok(&selectors[index - 1])
}

fn first_signature(info: &SelectorInfo) -> &str {
    info.signatures
        .first()
        .map(String::as_str)
        .unwrap_or("<unknown>")
}

fn extract_arg_types(signature: &str) -> Vec<String> {
    let Some(start) = signature.find('(') else {
        return vec![];
    };

    let Some(end) = signature.rfind(')') else {
        return vec![];
    };

    let inside = &signature[start + 1..end];

    if inside.trim().is_empty() {
        return vec![];
    }

    inside
        .split(',')
        .map(|s| s.trim().to_string())
        .collect()
}

fn ask_function_args(signature: &str) -> Result<Vec<String>> {
    let arg_types = extract_arg_types(signature);
    let mut user_args = Vec::new();

    for (i, arg_type) in arg_types.iter().enumerate() {
        let value = ask(&format!("arg{} {}: ", i, arg_type))?;
        user_args.push(value);
    }

    Ok(user_args)
}

fn execution_mode(info: &SelectorInfo) -> Result<&'static str> {
    match info.mutability.as_deref().unwrap_or("unknown") {
        "view" | "pure" => Ok("call"),
        "nonpayable" | "payable" => Ok("send"),
        _ => {
            let manual_mode = ask("Unknown mutability. Choose mode [call/send]: ")?;

            match manual_mode.as_str() {
                "call" => Ok("call"),
                "send" => Ok("send"),
                _ => anyhow::bail!("invalid mode"),
            }
        }
    }
}

fn execute_function(
    address: &str,
    rpc_url: &str,
    info: &SelectorInfo,
) -> Result<()> {
    let signature = first_signature(info);

    println!("\nSelected function:");
    println!("{}", signature);

    if signature == "<unknown>" {
        anyhow::bail!("cannot execute unknown function signature");
    }

    let user_args = ask_function_args(signature)?;
    let mode = execution_mode(info)?;

    let mut cmd_args = vec![
        mode.to_string(),
        address.to_string(),
        signature.to_string(),
    ];

    cmd_args.extend(user_args);

    cmd_args.push("--rpc-url".to_string());
    cmd_args.push(rpc_url.to_string());

    println!("\nRunning:");
    println!("cast {}", cmd_args.join(" "));

    let cmd_refs: Vec<&str> = cmd_args.iter().map(String::as_str).collect();
    let output = cast(&cmd_refs)?;

    println!("\nOutput:");
    println!("{}", output);

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let selectors = scan_contract(&args.address, &args.rpc_url)?;

    display_functions(&selectors);

    let selected = choose_function(&selectors)?;

    execute_function(&args.address, &args.rpc_url, selected)?;

    Ok(())
}