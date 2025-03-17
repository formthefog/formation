# Formation API OpenAPI Specifications

This section contains OpenAPI 3.0 specifications for the Formation Protocol APIs. OpenAPI (formerly known as Swagger) is a standardized format for describing RESTful APIs that facilitates both human and machine readability.

## Available Specifications

- [State Service API](./state-api.yaml): OpenAPI specification for the State Service API, which maintains the globally consistent state of the Formation cloud.

## Using the OpenAPI Specifications

These OpenAPI specifications can be used in various ways:

### 1. API Documentation

Import the YAML files into tools like [Swagger UI](https://swagger.io/tools/swagger-ui/) or [Redoc](https://github.com/Redocly/redoc) to generate interactive API documentation.

### 2. Code Generation

Use tools like [OpenAPI Generator](https://openapi-generator.tech/) to generate client libraries for various programming languages:

```bash
# Install OpenAPI Generator
npm install @openapitools/openapi-generator-cli -g

# Generate a JavaScript client for the State Service API
openapi-generator-cli generate -i state-api.yaml -g javascript -o ./formation-client-js
```

### 3. API Testing

Import the specifications into API testing tools like [Postman](https://www.postman.com/) or [Insomnia](https://insomnia.rest/) to create request collections automatically.

## Specification Format

All specifications follow the OpenAPI 3.0.3 format and include:

- API endpoints with paths and methods
- Request parameters and body schemas
- Response schemas
- Error response definitions
- Object models
- Authentication requirements

## Contributing to the Specifications

When making changes to the Formation APIs, please update the corresponding OpenAPI specifications to ensure they remain accurate and up-to-date.

## Future Plans

We plan to add OpenAPI specifications for the following APIs:

- VMM Service API
- P2P Service API
- DNS Service API
- Formnet API
- Inference Engine API 