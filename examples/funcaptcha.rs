use openai::arkose;
use openai::arkose::funcaptcha::solver::ArkoseSolver;
use openai::arkose::funcaptcha::solver::Solver;
use openai::arkose::ArkoseContext;
use openai::arkose::ArkoseToken;
use openai::context::args::Args;
use openai::{
    context::{self},
    with_context,
};
use std::str::FromStr;
use tokio::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let client_key = std::env::var("KEY").expect("Need solver client key");
    let solver = Solver::from_str(&std::env::var("SOLVER").expect("Need solver"))
        .expect("Not support solver");
    let solver_type = std::env::var("SOLVER_TYPE").expect("Need solver type");

    context::init(
        Args::builder()
            .arkose_solver_tguess_endpoint(Some("https://example.com/tguess".to_owned()))
            .arkose_solver(ArkoseSolver::new(solver, client_key, None, 1))
            .build(),
    );

    let typed = match solver_type.as_str() {
        "auth" => arkose::Type::Auth,
        "platform" => arkose::Type::Platform,
        "signup" => arkose::Type::SignUp,
        "gpt3" => arkose::Type::GPT3,
        "gpt4" => arkose::Type::GPT4,
        _ => anyhow::bail!("Not support solver type: {solver_type}"),
    };

    // start time
    let now = Instant::now();

    let arkose_token = ArkoseToken::new_from_context(
        ArkoseContext::builder()
            .client(with_context!(arkose_client))
            .typed(typed)
            .build(),
    )
    .await?;

    println!("Arkose token: {}", arkose_token.json());

    println!("Function execution time: {:?}", now.elapsed());
    Ok(())
}
