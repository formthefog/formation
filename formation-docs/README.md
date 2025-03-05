# Formation Protocol Documentation

This repository contains the comprehensive documentation for the Formation Protocol - a public verifiable and self-replicating protocol for trustless, confidential virtual private servers (VPS) coordinating as a Fog Compute network to power the Age of Autonomy.

## Structure

The documentation is organized into the following sections:

- **Operator Docs**: Information for node operators, including hardware requirements, setup, and management.
- **Developer Docs**: Information for developers building applications on the Formation cloud.
- **Architecture**: Technical details about the Formation cloud architecture.
- **Inference Engine**: Details about the AI inference capabilities (upcoming).
- **Pricing**: Information about costs and pricing models.
- **API Reference**: Comprehensive documentation of all API endpoints in the Formation ecosystem.

## Architecture Diagrams

The documentation includes comprehensive architecture diagrams built with Mermaid, a markdown-based diagramming tool. These diagrams follow the C4 model:

- **Level 1 (Context)**: Shows Formation in relation to its users and external systems
- **Level 2 (Containers)**: Shows the high-level components of the system
- **Level 3 (Components)**: Shows the internal structure of each container
- **Level 4 (Dynamic View)**: Shows the interactions between components during key operations

You can find the source code for these diagrams in the `docs/assets/diagrams/source` directory.

## Local Development

### Prerequisites

- Node.js version 16 or above

### Installation

```bash
# Clone the repository
git clone https://github.com/formthefog/formation-docs.git
cd formation-docs

# Install dependencies
npm install

# Start local development server
npm start
```

## Building

```bash
npm run build
```

## Deployment

The documentation is automatically deployed using GitHub Pages from the `gh-pages` branch.

## Contributing

Contributions to improve the documentation are welcome. Please follow these steps:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## License

This documentation is licensed under the same license as the Formation Protocol itself. 