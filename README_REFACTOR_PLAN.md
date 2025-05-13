# README Refactor Plan

## I. Overall README Structure Revision

- [ ] **Task 1: Review and Update Introduction/Overview.**
    - [ ] Ensure the "Overview" section accurately reflects the project's current state as a "Vertically Integrated 2-Sided Marketplace for AI Agents & Models."
    - [ ] Briefly mention the core services currently deployed via `docker-compose.yml` (`form-state`, `form-dns`, `form-net`, `form-vmm`, `form-pack-manager`).
- [ ] **Task 2: Condense or Remove Outdated Sections.**
    - [ ] Review "Marketplace Features," "Deployment Architecture," "Docker Configuration" (the `docker run` command), and "Special Thanks" for relevance and brevity.
    - [ ] Replace old "Running a Node" sections with `docker-compose` approach.
- [ ] **Task 3: Streamline Prerequisites & Setup.**
    - [ ] Verify system requirements and dependencies.
    - [ ] Keep Rust and Docker installation instructions.
    - [ ] Integrate network configuration (`scripts/validate-network-config.sh` and manual bridge setup) as a crucial prerequisite step before `docker-compose`.

## II. New Core Content - Deployment and Usage

- [ ] **Task 4: Add "Deploying Core Services with Docker Compose" Section.**
    - [ ] Explain the role of `docker-compose.yml`.
    - [ ] Step-by-step instructions:
        - [ ] Cloning the repository.
        - [ ] Running `scripts/validate-network-config.sh` (explain its purpose).
        - [ ] Alternatively, provide manual steps for bridge and network setup.
        - [ ] Explain `.env` file creation/configuration for `SECRET_PATH`, `PASSWORD`.
        - [ ] Running `docker-compose up -d`.
        - [ ] Verifying services are running (e.g., `docker ps`, health checks).
- [ ] **Task 5: Add "Deploying Your First Agent (via API)" Section.**
    - [ ] Explain this will use `curl` examples.
    - [ ] **Sub-task 5.1: Agent Creation Concept:**
        - [ ] Briefly explain "agent" as a VM.
        - [ ] Mention `Formfile` conceptually.
        - [ ] Explain `form-pack-manager` role.
    - [ ] **Sub-task 5.2: API Interaction - Creating an Instance (Agent VM):**
        - [ ] Target `form-state`'s `/instance/create` endpoint.
        - [ ] Explain how to construct the `CreateInstanceRequest` JSON payload (from `form-state/src/helpers/instances.rs`).
        - [ ] Show `curl` example to `form-state`'s `/instance/create` endpoint.
        - [ ] Explain ECDSA signature authentication (`X-Signature`, `X-Recovery-Id`, `X-Message` headers based on `form-state/src/auth.rs`).
    - [ ] **Sub-task 5.3: API Interaction - Checking Agent Status/Boot Completion:**
        - [ ] Target `form-state`'s `/instance/:instance_id/get` or `/instance/:build_id/get_by_build_id`.
        - [ ] Show `curl` example.
    - [ ] **Sub-task 5.4: API Interaction - Interacting with the Deployed Agent:**
        - [ ] Explain interaction with the agent's service on its Formnet IP.
        - [ ] Provide a hypothetical `curl` example to the agent's service.

## III. Roadmap Section

- [ ] **Task 6: Develop "Project Roadmap" Section.**
    - [ ] **Production Ready:**
        - [ ] Core services: `form-state`, `form-dns`, `form-net`, `form-vmm`, `form-pack-manager`.
        - [ ] Deployment via `docker-compose`.
        - [ ] Basic VM lifecycle management via API.
    - [ ] **Under Construction (Nearing Production Readiness):**
        - [ ] Refined API authentication/authorization.
        - [ ] Agent interaction patterns.
        - [ ] `form-cli` (needs fixes).
        - [ ] Monitoring: `form-node-metrics`, `form-vm-metrics` (integration).
        - [ ] Advanced networking: `form-bgp` (integration).
        - [ ] Full marketplace features in `form-state`.
    - [ ] **Planned (Future Work):**
        - [ ] Comprehensive documentation site.
        - [ ] Full `form-p2p` event-driven message queue integration.
        - [ ] Enhanced security (TEE, HSM).
        - [ ] Wider AI model/agent framework support.
        - [ ] Marketplace UI.

## IV. Final Touches

- [ ] **Task 7: Review and Update "Contributing" and "Pre-release Notice".**
- [ ] **Task 8: Remove Obsolete Sections/Comments.**
    - [ ] Remove `formation-docs` and `docs` directory mentions.
    - [ ] Omit `form-broker` from active services list in README, noting it's a build dependency but not active. 