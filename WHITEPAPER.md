# GhostCoin (GHST) — Whitepaper v1.0.0

## Abstract

GhostCoin est une cryptomonnaie axée sur la vie privée,
combinant les meilleures technologies de confidentialité
disponibles aujourd'hui pour garantir des transactions
totalement privées et intraçables.

---

## 1. Introduction

Les blockchains publiques traditionnelles (Bitcoin, Ethereum)
exposent toutes les transactions publiquement. GhostCoin résout
ce problème en combinant 5 couches de privacy.

---

## 2. Spécifications Techniques

| Paramètre       | Valeur                    |
|-----------------|---------------------------|
| Nom             | GhostCoin                 |
| Symbole         | GHST                      |
| Supply Maximum  | 50,000,000 GHST           |
| Récompense bloc | 65 GHST                   |
| Halving         | Tous les 210,000 blocs    |
| Temps par bloc  | ~2 minutes                |
| Algorithme PoW  | SHA-256                   |
| Courbe          | Ristretto255 / BLS12-381  |

---

## 3. Architecture Privacy

### 3.1 Stealth Addresses
Chaque transaction génère une adresse unique et jetable.
Le destinataire est le seul à pouvoir identifier ses paiements.

### 3.2 Ring Signatures
L'expéditeur signe au nom d'un groupe de clés publiques.
Il est impossible de déterminer qui a réellement signé.

### 3.3 Confidential Transactions
Les montants sont chiffrés via Pedersen Commitments.
Le réseau vérifie la balance sans voir les chiffres.

### 3.4 zk-SNARKs (Groth16)
Preuves mathématiques de validité sans révélation de données.
Basé sur la courbe BLS12-381 (même technologie que Zcash).

### 3.5 Dandelion++
Protection de l'adresse IP de l'expéditeur.
Les transactions passent par des noeuds relais aléatoires
avant d'être broadcastées au réseau complet.

---

## 4. Tokenomics

- Supply total : 50,000,000 GHST
- Distribution : 100% par minage (aucune premine)
- Récompense initiale : 65 GHST par bloc
- Halving : tous les 210,000 blocs (~2 ans)

---

## 5. Réseau

- Protocole : TCP P2P async (Tokio/Rust)
- Propagation : Dandelion++  
- Consensus : Proof of Work (SHA-256)
- Résolution conflits : Longest chain rule

---

## 6. Roadmap

- [x] v1.0 — Blockchain + Privacy features
- [x] v1.1 — Réseau P2P + Mempool
- [x] v1.2 — Consensus PoW
- [x] v1.3 — CLI Wallet + Dandelion++
- [ ] v1.4 — Déploiement mainnet
- [ ] v1.5 — Interface graphique
- [ ] v1.6 — Wallet mobile
- [ ] v2.0 — Exchange listings

---

## 7. Conclusion

GhostCoin représente l'état de l'art en matière de
confidentialité blockchain, combinant les meilleures
technologies disponibles dans une implémentation
100% open source en Rust.

**Symbol** : GHST
**Website** : ghostcoin.network (à venir)
**GitHub**  : github.com/ghostcoin (à venir)