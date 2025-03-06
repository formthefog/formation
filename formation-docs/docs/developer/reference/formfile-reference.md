---
id: formfile-reference
title: Formation Formfile Reference
sidebar_label: Formfile Reference
---

# Formation Formfile Reference

This document provides a comprehensive reference for all Formfile directives and syntax. The Formfile is the core configuration file used to define your virtual machine instance in Formation.

# Formfile Reference

This document provides a comprehensive reference for all Formfile directives and syntax. The Formfile is the core configuration file used to define your virtual machine instance in Formation.

## Basic Structure

A Formfile consists of a series of directives, each on a new line. Comments can be added with `#` and are ignored during parsing.

```
# This is a comment
NAME my-instance  # This is also a comment
```

## Core Directives

### NAME

**Syntax**: `NAME <name>`

**Description**: Sets the name of your application instance. This name is used for identification in the Formation cloud.

**Example**:
```
NAME web-server
```

**Notes**:
- Names should be unique within your account
- Names should only contain alphanumeric characters, dashes, and underscores
- Names are case-sensitive

### USER

**Syntax**: `USER username:<username> passwd:<password> sudo:<true|false> ssh_authorized_keys:"<ssh-public-key>"`

**Description**: Configures the user account for the instance.

**Example**:
```
USER username:admin passwd:secure123 sudo:true ssh_authorized_keys:"ssh-rsa AAAAB3NzaC1yc2EAA..."
```

**Parameters**:
- `username`: The user's login name (required)
- `passwd`: Password for the user (optional if SSH keys are provided)
- `sudo`: Whether to grant sudo permissions (`true` or `false`)
- `ssh_authorized_keys`: SSH public keys for authentication (recommended)

**Notes**:
- For production instances, it's recommended to use SSH keys rather than passwords
- Multiple SSH keys can be provided as a comma-separated list
- Password complexity requirements apply

### VCPU

**Syntax**: `VCPU <count>`

**Description**: Specifies the number of virtual CPU cores to allocate to the instance.

**Example**:
```
VCPU 2
```

**Notes**:
- Valid values: 1-128 (upper limit depends on operator node capabilities)
- Resource availability may affect actual allocation

### MEM

**Syntax**: `MEM <size_in_megabytes>`

**Description**: Specifies the memory allocation for the instance in megabytes.

**Example**:
```
MEM 4096  # 4GB RAM
```

**Notes**:
- Minimum: 512MB
- Maximum: Depends on operator node capabilities
- Recommended to be a multiple of 1024

### DISK

**Syntax**: `DISK <size_in_gigabytes>`

**Description**: Specifies the disk size for the instance in gigabytes.

**Example**:
```
DISK 10  # 10GB disk
```

**Notes**:
- Minimum: 5GB
- Maximum: Depends on operator node capabilities

## Base Image Directives

### FROM

**Syntax**: `FROM <image>:<tag>`

**Description**: Specifies the base operating system image to use.

**Example**:
```
FROM ubuntu:22.04
```

**Available Images**:
- `ubuntu:22.04` (default)
- `ubuntu:20.04`
- `debian:11`
- `alpine:3.16`

**Notes**:
- If not specified, defaults to `ubuntu:22.04`
- Custom images will be supported in future versions

## Filesystem Directives

### COPY

**Syntax**: `COPY <source> <destination>`

**Description**: Copies files or directories from your local context to the instance.

**Example**:
```
COPY ./app /app
COPY ./config/nginx.conf /etc/nginx/nginx.conf
```

**Options**:
- `--from=<stage>`: Copy from a previous multi-stage build (for multi-stage builds)

**Notes**:
- Paths are relative to the build context
- Wildcards are supported (e.g., `COPY ./config/*.conf /etc/nginx/conf.d/`)
- Preserves file permissions

### WORKDIR

**Syntax**: `WORKDIR <path>`

**Description**: Sets the working directory for subsequent RUN, CMD, ENTRYPOINT instructions.

**Example**:
```
WORKDIR /app
```

**Notes**:
- Creates the directory if it doesn't exist
- Can be used multiple times to change the working directory

## Execution Directives

### RUN

**Syntax**: `RUN <command>`

**Description**: Executes commands during the build process.

**Example**:
```
RUN apt-get update && apt-get install -y nginx
```

**Notes**:
- Commands run with `/bin/sh -c` by default
- Use `&&` to chain commands
- Complex commands can be continued on multiple lines with `\`

### INSTALL

**Syntax**: `INSTALL <package1> <package2> ...`

**Description**: Installs packages using the system package manager.

**Example**:
```
INSTALL nginx postgresql python3
```

**Notes**:
- Automatically uses the appropriate package manager (apt, yum, apk)
- Implicitly runs update before installation
- Recommended over `RUN apt-get install` for better caching

### ENTRYPOINT

**Syntax**: `ENTRYPOINT ["executable", "param1", "param2"]` (exec form, recommended)
**Alternative Syntax**: `ENTRYPOINT command param1 param2` (shell form)

**Description**: Configures the command that will run when the instance starts.

**Example**:
```
ENTRYPOINT ["nginx", "-g", "daemon off;"]
```

**Notes**:
- Exec form is preferred for better signal handling
- Only the last ENTRYPOINT directive is effective

### CMD

**Syntax**: `CMD ["executable", "param1", "param2"]` (exec form, recommended)
**Alternative Syntax**: `CMD command param1 param2` (shell form)

**Description**: Provides default arguments for the ENTRYPOINT.

**Example**:
```
CMD ["--port", "8080"]
```

**Notes**:
- Can be overridden at runtime
- If ENTRYPOINT is not specified, CMD specifies the command to run

## Environment Directives

### ENV

**Syntax**: `ENV <key>=<value> ...`

**Description**: Sets environment variables in the instance.

**Example**:
```
ENV NODE_ENV=production
ENV DB_HOST=localhost DB_PORT=5432
```

**Notes**:
- Values with spaces should be quoted
- Variables are available during build and at runtime
- Can set multiple variables in a single directive

## Networking Directives

### EXPOSE

**Syntax**: `EXPOSE <port> [<port>...]`

**Description**: Exposes ports for networking.

**Example**:
```
EXPOSE 80 443
```

**Notes**:
- Does not publish the ports to the host
- Used for documentation and to indicate which ports are intended to be published

## Advanced Directives

### VOLUME

**Syntax**: `VOLUME ["<path>", "<path>", ...]` or `VOLUME <path> [<path>...]`

**Description**: Creates a mount point for externally mounted volumes.

**Example**:
```
VOLUME ["/data"]
```

**Notes**:
- Volumes are populated with the contents of the specified directory
- Volumes persist even when instances are stopped
- Future releases will support volume management commands

### HEALTHCHECK

**Syntax**: `HEALTHCHECK [OPTIONS] CMD <command>`

**Description**: Configures a command to check container health.

**Example**:
```
HEALTHCHECK --interval=5m --timeout=3s CMD curl -f http://localhost/ || exit 1
```

**Options**:
- `--interval=<duration>`: Time between health checks (default: 30s)
- `--timeout=<duration>`: Maximum time for a health check to complete (default: 30s)
- `--retries=<n>`: Number of consecutive failures required to report unhealthy (default: 3)

**Notes**:
- Exit code 0: success, container is healthy
- Exit code 1: failure, container is unhealthy
- Any other exit code: error running health check

### STOPSIGNAL

**Syntax**: `STOPSIGNAL <signal>`

**Description**: Sets the system call signal to be sent to the instance to exit.

**Example**:
```
STOPSIGNAL SIGTERM
```

**Notes**:
- Valid signals: SIGTERM, SIGINT, SIGKILL, etc.
- Default: SIGTERM

## Multi-stage Builds

Formation supports multi-stage builds, allowing you to use multiple FROM directives in your Formfile. Each FROM directive begins a new stage of the build.

**Example**:
```
# Stage 1: Build the application
NAME build-stage
FROM ubuntu:22.04
INSTALL gcc make
COPY ./src /src
WORKDIR /src
RUN make

# Stage 2: Create the production instance
NAME production-stage
FROM ubuntu:22.04
COPY --from=build-stage /src/bin /app
WORKDIR /app
ENTRYPOINT ["./myapp"]
```

**Notes**:
- Use `--from=<stage>` with COPY to copy files from a previous stage
- Only the final stage is deployed as an instance
- Useful for creating smaller production images

## Best Practices

1. **Order instructions for caching efficiency**:
   - Place instructions that change less frequently at the top
   - Group related instructions
   - Place frequently changing instructions (like COPY of application code) last

2. **Use multi-stage builds** to minimize instance size

3. **Avoid unnecessary packages** and clean up after package installation

4. **Combine RUN instructions** using `&&` to reduce layer count

5. **Use ENTRYPOINT for main commands** and CMD for default parameters

6. **Prefer COPY over ADD** unless you specifically need the tar extraction features of ADD

7. **Set appropriate EXPOSE ports** for all services your application provides

8. **Document with comments** for clarity

## Examples

### Web Server Example

```
NAME web-server
USER username:webuser passwd:webpass123 sudo:true
VCPU 2
MEM 2048
DISK 10
INSTALL nginx
COPY ./www /var/www/html
COPY ./nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80 443
```

### Node.js Application Example

```
NAME nodejs-api
USER username:nodeuser passwd:nodepass123 sudo:true
VCPU 2
MEM 4096
DISK 20

INSTALL nodejs npm

WORKDIR /app
COPY package*.json ./
RUN npm install

COPY . .
ENV NODE_ENV=production
ENV PORT=3000

EXPOSE 3000
ENTRYPOINT ["node", "server.js"]
```

### Database Server Example

```
NAME postgres-db
USER username:pguser passwd:pgpass123 sudo:true
VCPU 4
MEM 8192
DISK 100

INSTALL postgresql

RUN mkdir -p /data/postgres && chown postgres:postgres /data/postgres
ENV PGDATA=/data/postgres

VOLUME ["/data/postgres"]
EXPOSE 5432

RUN echo "listen_addresses = '*'" >> /etc/postgresql/*/main/postgresql.conf
RUN echo "host all all 0.0.0.0/0 md5" >> /etc/postgresql/*/main/pg_hba.conf

USER postgres
```

### Multi-stage Build Example

```
# Build stage
NAME build-stage
VCPU 4
MEM 8192
INSTALL golang

WORKDIR /go/src/app
COPY . .
RUN go mod download
RUN GOOS=linux GOARCH=amd64 go build -o myapp

# Production stage
NAME myapp-production
VCPU 2
MEM 2048
DISK 10

COPY --from=build-stage /go/src/app/myapp /app/
COPY --from=build-stage /go/src/app/config.yaml /app/

WORKDIR /app
ENTRYPOINT ["./myapp"]
EXPOSE 8080
``` 