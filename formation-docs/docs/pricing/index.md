# Formation Pricing and Tokenomics

Formation offers a transparent pricing model designed to fairly compensate resource providers while keeping costs predictable for developers and users. This document outlines our pricing structure, tokenomics, and economic model.

## Pricing Philosophy

Formation's pricing is built on these principles:

1. **Transparent Billing**: Clear, predictable costs with no hidden fees
2. **Fair Compensation**: Node operators are fairly compensated for resources provided
3. **Resource-Based**: Costs reflect actual resource consumption
4. **Efficiency Incentives**: Cost structures that encourage efficient resource use

## Compute Resource Pricing

### Standard Compute

The basic compute unit pricing is based on these resources:

| Resource | Unit | Price per Hour |
|----------|------|---------------|
| vCPU | 1 Core | $0.002 |
| Memory | 1 GB | $0.002 |
| Storage | 1 GB | $0.0001 |
| GPU | model dependent | N/A |

For example, a standard instance with 2 vCPU, 4GB RAM, and 20GB storage would cost approximately:
- (2 × $0.002) + (4 × $0.002) + (20 × $0.0001) = $0.014 per hour
- Approximately $10.08 per month (without network transfer)

### GPU Compute

For AI workloads and other GPU-accelerated applications:

| GPU Type | Price per Hour |
|----------|---------------|
| RTX5090 | $0.40-$0.60 |
| H100 | $1.20-$1.60 |
| H200 | $2.00-$2.50 |
| B200 | $3.50-$4.00 |

GPU instances also include the standard compute resources needed to run them.

## Inference Engine Pricing

The Inference Engine uses a token-based pricing model:

| Model Size |Price (per 1 Million tokens) |
|------------|------------------------------|
| < 32B Parameters | $0.1 |
| 32B - 110B Parameters | $0.2 |
| 110B - 500B Parameters | $0.3 |
| > 500B Parameters | $0.4 |
### Token Counting

Token counting follows industry standards:

- For English text, approximately 4 characters or ~0.75 words per token
- For images in vision models, pricing is based on resolution tiers
- For embeddings, only input tokens are charged

## Payment Methods

### Formation Credits

- Purchase credits using fiat currency or cryptocurrency
- Credits are used to pay for all Formation services
- Volume discounts available for large credit purchases

### Cryptocurrency

- Direct payment via Ethereum and other major cryptocurrencies
- Automatic conversion to credits at current market rates
- Gas fees are separate and not included in service pricing

## Tokenomics

> **Note**: The tokenomics model is currently under development and subject to change.

Formation's native token (FORM) is designed to:

1. **Incentivize Node Operators**: Reward those who provide compute resources
2. **Enable Governance**: Allow stakeholders to participate in protocol decisions
3. **Provide Economic Security**: Secure the network through staking requirements
4. **Create Network Effects**: Reward early adopters and active participants

### Token Distribution

The planned distribution of FORM tokens:

- **20%**: Node Operator Rewards
- **20%**: Development Fund
- **10%**: Early Investors
- **25%**: Community Growth
- **25%**: Team and Advisors

### Staking Requirements

Node operators must stake ETH or restaked ETH to participate in the network:

- **Minimum Stake**: 10% of annual value of capacity provided
- **Rewards**: Operators receive FORM tokens based on resources provided and uptime
- **Slashing**: Penalties for violating protocol rules or failing to meet SLAs

## Discounts and Incentives

### Volume Discounts

| Monthly Spend | Discount |
|---------------|----------|
| $1,000 - $10,000 | 5% |
| $10,001 - $100,000 | 10% |
| $100,001 - $1,000,000 | 15% |
| $1,000,001+ | Custom pricing |

### Commitment Discounts

| Commitment Term | Additional Discount |
|-----------------|---------------------|
| 3 Months | 5% |
| 6 Months | 10% |
| 12 Months | 15% |

### Early Adopter Program

Early adopters receive:

- **Credits**: $120 in free credits
- **Rate Lock**: Guaranteed pricing for 12 months
- **Priority Support**: Enhanced support services
- **FORM Tokens**: Allocation of FORM tokens upon network launch

## SLAs and Guarantees

Formation offers all customers a single Service Level Agreement:

| Service Tier | Uptime Guarantee | Compensation |
|--------------|------------------|--------------|
| Standard | 100% | 10% credit for every 0.1% below SLA |

## Billing and Invoicing

### Billing Cycles

- **Subscription**: All users beyond the free tier are billed monthly for a minimum number of credits.
- **Prepaid Credits**: Purchase credits in advance with volume discounts.

## Cost Optimization

Formation provides tools to help optimize costs:

- **Resource Monitoring**: Track resource usage in real-time
- **Budgeting Tools**: Set spending limits and alerts
- **Elastic Scaling**: Automatically scale resources up and down based on demand
- **Idle Detection**: Identify and hibernate underutilized instances
- **Automated Failover**: Automatically failover to a new node if the current node is unhealthy
- **Geographic Redundancy**: All instances are deployed across multiple geographic regions automatically at no extra cost. Default is 3 redundancies, but can be increased at an extra fee for each additional instance.
- **Fully Customizable**: Formation instances are proper Virtual Private Servers (Linux VMs) with full root access.
- **Trustless Security**: Formation instance images are encrypted and only accessible by authorized users.

## Enterprise Pricing

For enterprise customers with specialized needs:

- **Custom SLAs**: Tailored service level agreements with smart contract based penalties for SLA violations.
- **Volume Discounts**: Significant discounts for large deployments
- **Private Network**: Dedicated resources and infrastructure
- **Contract Flexibility**: Customized terms and conditions

Contact our [sales team](mailto:enterprise@formation.cloud) for enterprise pricing.

## Pricing Updates

Formation strives to maintain stable pricing, but may update prices to reflect market conditions:

- Existing customers receive 90 days notice before price changes
- Commitment plans lock in rates for the duration of the commitment
- Historical pricing data is maintained for reference

## Frequently Asked Questions

### How am I billed for partial usage?

Resources are billed by the second, with a minimum charge of one minute.

### Can I convert between different payment methods?

Yes, you can convert between fiat, crypto, and credits at any time using current exchange rates.

### How do I estimate my monthly costs?

Use our [pricing calculator](https://formation.cloud/pricing/calculator) to estimate costs based on your expected resource usage.

### Are there any hidden fees?

No. Formation is committed to transparent pricing with no hidden fees or charges. 