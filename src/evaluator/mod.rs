pub mod scorer;
pub mod risk_analyzer;
pub mod decision_maker;

pub use scorer::OpportunityScorer;
pub use risk_analyzer::RiskAnalyzer;
pub use decision_maker::DecisionMaker;

#[derive(Debug, Clone)]
pub struct OpportunityScore {
    pub mev_score: f64,
    pub confidence: f64,
    pub profitability: f64,
    pub risk_level: u8,
}