pub mod eval;
pub mod lex;
pub mod parse;
pub mod print;

pub fn eval(input: &str) -> Result<String, Vec<parse::Error>> {
    let block_lexer = lex::BlockLexer::new(input);
    let block_tokens = block_lexer.collect::<Vec<_>>();

    let lexer = lex::Lexer::new(block_tokens.into_iter());
    let tokens = lexer.collect::<Vec<_>>();

    let mut parser = parse::Parser::new(tokens.into_iter());
    let program_in = parser.program();

    let mut evaluator = eval::Evaluator::new();
    let program_out = evaluator.eval(&program_in);

    match program_out {
        Ok(program_out) => {
            let mut printer = print::Printer::new();
            Ok(printer.print(&program_out))
        }
        Err(e) => Err(e),
    }
}
