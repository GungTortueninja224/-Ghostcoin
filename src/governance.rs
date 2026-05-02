use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const GOV_FILE: &str = "ghostcoin_governance.json";
pub const MIN_VOTE_GHST: u64 = 10; // minimum 10 GHST pour voter

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Expired,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vote {
    pub voter: String,
    pub choice: VoteChoice,
    pub weight: u64, // poids du vote = solde GHST
    pub time: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Proposal {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub proposer: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub votes: Vec<Vote>,
    pub status: ProposalStatus,
    pub category: String,
}

impl Proposal {
    pub fn new(id: u64, title: &str, description: &str, proposer: &str, category: &str) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id,
            title: title.to_string(),
            description: description.to_string(),
            proposer: proposer.to_string(),
            created_at: now,
            expires_at: now + (7 * 24 * 3600), // 7 jours
            votes: vec![],
            status: ProposalStatus::Active,
            category: category.to_string(),
        }
    }

    pub fn vote(&mut self, voter: &str, choice: VoteChoice, weight: u64) -> bool {
        if self.status != ProposalStatus::Active {
            println!("❌ Proposition fermée");
            return false;
        }

        if Utc::now().timestamp() > self.expires_at {
            self.status = ProposalStatus::Expired;
            println!("❌ Proposition expirée");
            return false;
        }

        // Un wallet = un vote
        if self.votes.iter().any(|v| v.voter == voter) {
            println!("❌ Tu as déjà voté pour cette proposition");
            return false;
        }

        self.votes.push(Vote {
            voter: voter.to_string(),
            choice,
            weight,
            time: Utc::now().timestamp(),
        });

        println!("✅ Vote enregistré !");
        true
    }

    pub fn yes_votes(&self) -> u64 {
        self.votes
            .iter()
            .filter(|v| v.choice == VoteChoice::Yes)
            .map(|v| v.weight)
            .sum()
    }

    pub fn no_votes(&self) -> u64 {
        self.votes
            .iter()
            .filter(|v| v.choice == VoteChoice::No)
            .map(|v| v.weight)
            .sum()
    }

    pub fn total_votes(&self) -> u64 {
        self.votes.iter().map(|v| v.weight).sum()
    }

    pub fn is_passed(&self) -> bool {
        let yes = self.yes_votes();
        let total = self.total_votes();
        total > 0 && yes * 100 / total > 50
    }

    pub fn finalize(&mut self) {
        if self.status != ProposalStatus::Active {
            return;
        }

        self.status = if self.is_passed() {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Rejected
        };
    }

    pub fn show(&self) {
        let status_str = match &self.status {
            ProposalStatus::Active => "🟢 Active",
            ProposalStatus::Passed => "✅ Adoptée",
            ProposalStatus::Rejected => "❌ Rejetée",
            ProposalStatus::Expired => "⏰ Expirée",
        };

        let yes = self.yes_votes();
        let no = self.no_votes();
        let total = self.total_votes();
        let pct = yes
            .checked_mul(100)
            .and_then(|votes| votes.checked_div(total))
            .unwrap_or(0);

        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║  🗳️  PROPOSITION #{:<49} ║", self.id);
        println!("╠══════════════════════════════════════════════════════════╣");
        println!(
            "║  Titre    : {:<44} ║",
            &self.title[..self.title.len().min(44)]
        );
        println!("║  Catégorie: {:<44} ║", self.category);
        println!("║  Status   : {:<44} ║", status_str);
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  ✅ Oui   : {:<44} ║", format!("{} GHST ({}%)", yes, pct));
        println!("║  ❌ Non   : {:<44} ║", format!("{} GHST", no));
        println!("║  Total    : {:<44} ║", format!("{} GHST", total));
        println!("║  Votants  : {:<44} ║", self.votes.len());
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}

// ==========================================
// GESTIONNAIRE DE GOUVERNANCE
// ==========================================
pub struct GovernanceManager {
    pub proposals: Vec<Proposal>,
}

impl GovernanceManager {
    pub fn load() -> Self {
        if !Path::new(GOV_FILE).exists() {
            return Self {
                proposals: Self::default_proposals(),
            };
        }
        let json = fs::read_to_string(GOV_FILE).unwrap_or_default();
        let proposals = serde_json::from_str(&json).unwrap_or_else(|_| Self::default_proposals());
        Self { proposals }
    }

    fn default_proposals() -> Vec<Proposal> {
        vec![
            Proposal::new(
                1,
                "Augmenter la taille des blocs",
                "Passer de 1MB à 2MB pour plus de TX par bloc",
                "genesis",
                "Technique",
            ),
            Proposal::new(
                2,
                "Réduire la difficulté de minage",
                "Difficulté 4 → 3 pour accélérer les blocs",
                "genesis",
                "Consensus",
            ),
            Proposal::new(
                3,
                "Ajouter support Tor obligatoire",
                "Rendre Tor obligatoire pour tous les noeuds",
                "genesis",
                "Privacy",
            ),
        ]
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(&self.proposals).unwrap();
        let _ = fs::write(GOV_FILE, json);
    }

    pub fn create_proposal(
        &mut self,
        title: &str,
        description: &str,
        proposer: &str,
        category: &str,
        balance: u64,
    ) -> bool {
        if balance < 100 {
            println!("❌ Minimum 100 GHST requis pour créer une proposition");
            return false;
        }

        let id = self.proposals.len() as u64 + 1;
        let p = Proposal::new(id, title, description, proposer, category);
        println!("✅ Proposition #{} créée !", id);
        self.proposals.push(p);
        self.save();
        true
    }

    pub fn vote(
        &mut self,
        proposal_id: u64,
        voter: &str,
        choice: VoteChoice,
        balance: u64,
    ) -> bool {
        if balance < MIN_VOTE_GHST {
            println!("❌ Minimum {} GHST requis pour voter", MIN_VOTE_GHST);
            return false;
        }

        match self.proposals.iter_mut().find(|p| p.id == proposal_id) {
            Some(proposal) => {
                let result = proposal.vote(voter, choice, balance);
                if result {
                    self.save();
                }
                result
            }
            None => {
                println!("❌ Proposition #{} non trouvée", proposal_id);
                false
            }
        }
    }

    pub fn show_all(&self) {
        println!(
            "\n🗳️  GHOSTCOIN GOUVERNANCE — {} proposition(s)",
            self.proposals.len()
        );
        for p in &self.proposals {
            p.show();
        }
    }

    pub fn active_proposals(&self) -> Vec<&Proposal> {
        self.proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::Active)
            .collect()
    }
}
