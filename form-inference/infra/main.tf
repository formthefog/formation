# Configure the AWS provider with the desired region
provider "aws" {
  region = "us-east-2"  # Change to your preferred region
}

# Define a Virtual Private Cloud (VPC) to host the network infrastructure
resource "aws_vpc" "pytorch_vpc" {
  cidr_block = "10.0.0.0/16"  # The IP range for the VPC
  enable_dns_support = true   # Enable DNS resolution within the VPC
  enable_dns_hostnames = true # Enable DNS hostnames for instances within the VPC
  
  tags = {
    Name = "pytorch-vpc"  # Tag for identifying the VPC
  }
}

# Create a subnet within the VPC for hosting resources
resource "aws_subnet" "pytorch_subnet" {
  vpc_id = aws_vpc.pytorch_vpc.id  # Associate the subnet with the VPC
  cidr_block = "10.0.1.0/24"       # The IP range for the subnet
  map_public_ip_on_launch = true   # Automatically assign a public IP to instances launched in this subnet
  
  tags = {
    Name = "pytorch-subnet"  # Tag for identifying the subnet
  }
}

# Set up an Internet Gateway to allow internet access for resources in the VPC
resource "aws_internet_gateway" "pytorch_igw" {
  vpc_id = aws_vpc.pytorch_vpc.id  # Associate the Internet Gateway with the VPC
  
  tags = {
    Name = "pytorch-igw"  # Tag for identifying the Internet Gateway
  }
}

# Create a Route Table to manage network traffic within the VPC
resource "aws_route_table" "pytorch_rt" {
  vpc_id = aws_vpc.pytorch_vpc.id  # Associate the Route Table with the VPC
  
  # Define a route to direct all outbound traffic to the Internet Gateway
  route {
    cidr_block = "0.0.0.0/0"  # Destination for all traffic
    gateway_id = aws_internet_gateway.pytorch_igw.id  # Use the Internet Gateway for outbound traffic
  }
  
  tags = {
    Name = "pytorch-rt"  # Tag for identifying the Route Table
  }
}

# Associate the Route Table with the Subnet to apply the routing rules
resource "aws_route_table_association" "a" {
  subnet_id = aws_subnet.pytorch_subnet.id  # The subnet to associate with the Route Table
  route_table_id = aws_route_table.pytorch_rt.id  # The Route Table to associate with the subnet
}

# Define a Security Group to control inbound and outbound traffic for instances
resource "aws_security_group" "pytorch_sg" {
  name = "pytorch-sg"  # Name of the Security Group
  description = "Allow traffic for PyTorch distributed"  # Description of the Security Group's purpose
  vpc_id = aws_vpc.pytorch_vpc.id  # Associate the Security Group with the VPC
  
  # Allow SSH access from anywhere
  ingress {
    from_port = 22  # Start of port range
    to_port = 22    # End of port range
    protocol = "tcp"  # Protocol type
    cidr_blocks = ["0.0.0.0/0"]  # Allow from any IP address
  }
  
  # Allow all internal traffic within the Security Group
  ingress {
    from_port = 0  # Start of port range
    to_port = 0    # End of port range
    protocol = "-1"  # All protocols
    self = true  # Allow traffic from within the Security Group
  }
  
  # Allow all outbound traffic to any destination
  egress {
    from_port = 0  # Start of port range
    to_port = 0    # End of port range
    protocol = "-1"  # All protocols
    cidr_blocks = ["0.0.0.0/0"]  # Allow to any IP address
  }
  
  tags = {
    Name = "pytorch-sg"  # Tag for identifying the Security Group
  }
}

# Launch a Master Node instance for PyTorch distributed training
resource "aws_instance" "master" {
  ami = "ami-0729f1db13c5d63f9"  # Deep Learning AMI with PyTorch (update for your region)
  instance_type = "c5.large"   # Instance type with GPU support
  key_name = "pytorch-test"     # SSH key pair for accessing the instance
  subnet_id = aws_subnet.pytorch_subnet.id  # Subnet to launch the instance in
  vpc_security_group_ids = [aws_security_group.pytorch_sg.id]  # Security Group for the instance
  
  tags = {
    Name = "pytorch-master"  # Tag for identifying the Master Node
  }
}

# Launch a Worker Node instance for PyTorch distributed training
resource "aws_instance" "worker" {
  ami = "ami-0729f1db13c5d63f9"  # Use the same AMI as the Master Node
  instance_type = "c5.large"   # Instance type with GPU support
  key_name = "pytorch-test"     # SSH key pair for accessing the instance
  subnet_id = aws_subnet.pytorch_subnet.id  # Subnet to launch the instance in
  vpc_security_group_ids = [aws_security_group.pytorch_sg.id]  # Security Group for the instance
  
  tags = {
    Name = "pytorch-worker"  # Tag for identifying the Worker Node
  }
}

# Output all the information needed
output "master_public_ip" {
  value = aws_instance.master.public_ip
  description = "Public IP of master node (for SSH access)"
}

output "worker_public_ip" {
  value = aws_instance.worker.public_ip
  description = "Public IP of worker node (for SSH access)"
}

output "master_private_ip" {
  value = aws_instance.master.private_ip
  description = "Private IP of master node (for PyTorch distributed communication)"
}

output "master_node_command" {
  value = "ssh -i /Users/bdimant/Developer/formation/form-inference/infra/pytorch-test.pem ubuntu@${aws_instance.master.public_ip}"
  description = "Command to SSH into master node"
}

output "worker_node_command" {
  value = "ssh -i /Users/bdimant/Developer/formation/form-inference/infra/pytorch-test.pem ubuntu@${aws_instance.worker.public_ip}"
  description = "Command to SSH into worker node"
}

output "master_run_command" {
  value = "MASTER_IP=${aws_instance.master.private_ip} NODE_RANK=0 ./run.sh"
  description = "Command to run on master node"
}

output "worker_run_command" {
  value = "MASTER_IP=${aws_instance.master.private_ip} NODE_RANK=1 ./run.sh"
  description = "Command to run on worker node"
}