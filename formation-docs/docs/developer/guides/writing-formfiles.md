# Writing Effective Formfiles

A `Formfile` is the core configuration file for your Formation deployment, similar to a Dockerfile in container workflows. It defines how your application will be packaged, configured, and deployed on the Formation cloud.

## Formfile Basics

A Formfile is a text file, typically named `Formfile` (no file extension) that should be placed at the root of your project directory. It contains a series of instructions that the Formation system will follow to build your instance.

## Formfile Syntax

Formfiles use a simple, declarative syntax with each instruction on a new line. Most instructions take the form of a keyword followed by arguments.

### Example Formfile

```
NAME my-web-application

USER username:webdev passwd:webpass123 sudo:true ssh_authorized_keys:"ssh-rsa AAAAB3NzaC1yc2EAAA..."

VCPU 2
MEM 4096
DISK 10

COPY ./app /app
INSTALL nodejs npm

WORKDIR /app
RUN npm install

ENTRYPOINT ["node", "server.js"]
```

## Core Instructions

### NAME

Defines the name of your application. This will be used for identification in the Formation cloud.

```
NAME my-application
```

### USER

Configures the user account for the instance:

```
USER username:myuser passwd:mypassword sudo:true ssh_authorized_keys:"ssh-rsa AAAAB3NzaC1yc2EAAA..."
```

Parameters:
- `username`: The username for the account
- `passwd`: The password (consider using SSH keys for production)
- `sudo`: Whether to grant sudo permissions (true/false)
- `ssh_authorized_keys`: SSH public keys for authentication

### VCPU

Specifies the number of virtual CPU cores:

```
VCPU 2
```

### MEM

Specifies the memory allocation in megabytes:

```
MEM 4096  # 4GB of RAM
```

### DISK

Specifies the disk size in gigabytes:

```
DISK 10  # 10GB of storage
```

### FROM

Specifies the base image to use (optional, defaults to Ubuntu 22.04):

```
FROM ubuntu:22.04
```

### COPY

Copies files or directories from your local context to the instance:

```
COPY ./local/path /destination/path
```

### INSTALL

Installs packages using the system package manager:

```
INSTALL python3 nginx postgresql
```

### RUN

Executes commands during the build process:

```
RUN mkdir -p /app/data && chmod 755 /app/data
```

### WORKDIR

Sets the working directory for subsequent RUN, CMD, ENTRYPOINT instructions:

```
WORKDIR /app
```

### ENV

Sets environment variables:

```
ENV NODE_ENV=production PORT=3000
```

### EXPOSE

Exposes ports for networking:

```
EXPOSE 80 443
```

### ENTRYPOINT

Configures the command that will run when the instance starts:

```
ENTRYPOINT ["python3", "app.py"]
```

## Advanced Usage

### Multi-stage Builds

You can use multi-stage builds to reduce the size of your final instance:

```
# Build stage
NAME build-stage

INSTALL nodejs npm
COPY ./app /app
WORKDIR /app
RUN npm install && npm run build

# Final stage
NAME my-production-app
COPY --from=build-stage /app/dist /app
WORKDIR /app
INSTALL nginx
EXPOSE 80
```

### Using Environment Variables

Environment variables can be set and used within the Formfile:

```
ENV DB_HOST=localhost
ENV DB_PORT=5432
RUN echo "Database connection: $DB_HOST:$DB_PORT" > /app/config.txt
```

### Optimizing Your Formfile

1. **Order matters**: Place infrequently changing instructions (like installing dependencies) before frequently changing ones (like copying your code) to leverage build caching.

2. **Minimize layers**: Combine related RUN commands using `&&` to reduce the number of layers and improve efficiency.

3. **Clean up**: Remove unnecessary files and packages to keep your instance lean.

## Best Practices

1. **Use specific versions** for your dependencies to ensure reproducible builds.

2. **Minimize instance size** by cleaning up temporary files after installation.

3. **Secure your instances** by avoiding hard-coded credentials and using environment variables instead.

4. **Document your Formfile** with comments to explain complex or non-obvious instructions.

5. **Test locally** before deploying to ensure your configuration works as expected.

## Formfile vs Dockerfile

If you're familiar with Docker, you'll notice similarities between Formfiles and Dockerfiles. However, there are some key differences:

- Formfiles create full VMs rather than containers
- Formation VMs have their own kernel and can run system services
- Formation provides better isolation and security guarantees
- Formation instances persist across reboots by default

## Example Formfiles

### Simple Web Server

```
NAME simple-web-server
USER username:webuser passwd:webpass sudo:true
VCPU 1
MEM 1024
DISK 5
INSTALL nginx
EXPOSE 80
RUN echo "Hello from Formation!" > /var/www/html/index.html
```

### Node.js Application

```
NAME nodejs-app
USER username:nodeuser passwd:nodepass sudo:true
VCPU 2
MEM 2048
DISK 10
INSTALL nodejs npm
COPY ./app /app
WORKDIR /app
RUN npm install
ENV PORT=3000
EXPOSE 3000
ENTRYPOINT ["node", "index.js"]
```

### Database Server

```
NAME postgres-db
USER username:pguser passwd:pgpass sudo:true
VCPU 4
MEM 8192
DISK 50
INSTALL postgresql
RUN mkdir -p /data/postgres && chown postgres:postgres /data/postgres
ENV PGDATA=/data/postgres
EXPOSE 5432
```

## Troubleshooting

### Common Issues

1. **Build Failures**: Check that all paths in COPY instructions are correct and that your build context includes all necessary files.

2. **Runtime Errors**: Ensure your ENTRYPOINT command is correct and that all required dependencies are installed.

3. **Network Issues**: Verify that you've used the EXPOSE instruction for any ports your application needs to communicate on.

### Debugging Tips

1. Use `form pack validate` to check your Formfile for syntax errors.

2. Use `form pack dry-run` to simulate the build process without actually deploying.

3. Check build logs with `form pack status --build-id <your-build-id>` for detailed error messages.

## Reference

For a complete reference of all Formfile instructions and options, see the [Formfile Reference](../reference/formfile-reference.md) documentation. 