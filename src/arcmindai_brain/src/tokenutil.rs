use tiktoken_rs::cl100k_base;

pub const MAX_16K_TOKENS: usize = 15000;

pub fn truncate_question(question: String, max_token_limit: usize) -> String {
    // check no. of tokens again
    let bpe = cl100k_base().unwrap();
    let tokens = bpe.encode_with_special_tokens(question.as_str());
    let tokens_len = tokens.len();
    ic_cdk::println!("Token count: : {}", tokens_len);

    if tokens_len > max_token_limit {
        let safe_question = question
            .chars()
            .take(question.len() / 2)
            .collect::<String>();
        ic_cdk::println!(
            "tokens_len reached limit {}!! Question is truncated to: \n{}",
            MAX_16K_TOKENS,
            safe_question
        );

        return truncate_question(safe_question, max_token_limit);
    }

    return question;
}
