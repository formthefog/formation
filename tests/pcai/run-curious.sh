#!/bin/bash

echo "pulling curious-ai frontend"
docker pull cryptonomikhan/curious-ai-frontend:latest
echo "pulling curious-ai backend"
docker pull cryptonomikhan/curious-ai-backend:latest
echo "pulling nginx"
docker pull nginx:latest 

echo "running curious-ai backend"
docker run -dit -p 0.0.0.0:8000:8000 cryptonomikhan/curious-ai-backend
echo "running curious-ai frontend"
docker run -dit -p 0.0.0.0:3000:3000 cryptonomikhan/curious-ai-frontend
echo "running nginx"
docker run -dit -p 0.0.0.0:80:80 -p 0.0.0.0:443:443 -v /opt/curious-ai/nginx/nginx.conf:/etc/nginx/nginx.conf:ro nginx:latest 
