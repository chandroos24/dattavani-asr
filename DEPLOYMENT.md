# Dattavani ASR Rust - Complete Deployment Documentation

This document provides comprehensive deployment instructions for the Dattavani ASR Rust application on AWS infrastructure.

## 📋 Table of Contents

1. [Infrastructure Overview](#infrastructure-overview)
2. [Prerequisites](#prerequisites)
3. [Quick Start](#quick-start)
4. [Detailed Deployment](#detailed-deployment)
5. [Configuration](#configuration)
6. [Monitoring & Maintenance](#monitoring--maintenance)
7. [Troubleshooting](#troubleshooting)
8. [Security](#security)
9. [Scaling](#scaling)
10. [Cost Optimization](#cost-optimization)

## 🏗️ Infrastructure Overview

### Existing AWS Resources

The deployment uses existing AWS infrastructure:

- **EC2 Instance**: `i-09726de87ad1f9596` (g4ad.2xlarge with GPU)
- **S3 Bucket**: `dattavani` (for media files and transcripts)
- **Security Group**: `sg-0264890af868ab040` (configured for SSH and HTTP)
- **IAM Instance Profile**: `Dattavani-Instance-Profile` (with S3 access)
- **Region**: `us-east-1`

### Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Internet      │    │   Load Balancer  │    │   EC2 Instance  │
│   (Users)       │───▶│   (Nginx)        │───▶│   (g4ad.2xlarge)│
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                        │
                                                        ▼
                                               ┌─────────────────┐
                                               │  Dattavani ASR  │
                                               │  Rust Service   │
                                               └─────────────────┘
                                                        │
                                                        ▼
                                               ┌─────────────────┐
                                               │   S3 Bucket     │
                                               │   (dattavani)   │
                                               └─────────────────┘
```

## 📋 Prerequisites

### Local Development Machine

1. **AWS CLI v2**
   ```bash
   # macOS
   brew install awscli
   
   # Linux
   curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
   unzip awscliv2.zip
   sudo ./aws/install
   ```

2. **AWS Credentials**
   ```bash
   aws configure
   # Enter your AWS Access Key ID, Secret Access Key, and region (us-east-1)
   ```

3. **SSH Key**
   - Ensure you have the SSH key for the EC2 instance
   - Add it to your SSH agent: `ssh-add ~/.ssh/your-key.pem`

4. **Rust (for local building)**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

### Verify Prerequisites

```bash
# Check AWS access
aws sts get-caller-identity

# Check instance access
aws ec2 describe-instances --instance-ids i-09726de87ad1f9596 --region us-east-1

# Check S3 access
aws s3 ls s3://dattavani/
```

## 🚀 Quick Start

### 1. Clone and Setup

```bash
# Clone the repository
git clone <repository-url> dattavani-asr-rust
cd dattavani-asr-rust

# Make deployment scripts executable
chmod +x aws-deployment/*.sh
```

### 2. Deploy to AWS

```bash
# Full automated deployment
./aws-deployment/deploy.sh

# Or step by step:
./aws-deployment/deploy.sh start    # Start instance
./aws-deployment/deploy.sh deploy   # Full deployment
```

### 3. Verify Deployment

```bash
# Check infrastructure status
./aws-deployment/manage-infrastructure.sh status

# SSH into the instance
./aws-deployment/deploy.sh ssh

# View logs
./aws-deployment/deploy.sh logs
```

## 📖 Detailed Deployment

### Step 1: Infrastructure Management

```bash
# Start the EC2 instance
./aws-deployment/manage-infrastructure.sh start

# Check status
./aws-deployment/manage-infrastructure.sh status

# Create backup (recommended before deployment)
./aws-deployment/manage-infrastructure.sh backup
```

### Step 2: Build and Deploy

```bash
# Build the project locally
cargo build --release

# Deploy to AWS
./aws-deployment/deploy.sh deploy
```

This script will:
- Start the EC2 instance if stopped
- Build the Rust application
- Upload the binary and configuration
- Install system dependencies
- Configure the service
- Start the application

### Step 3: Verify Deployment

```bash
# Get instance public IP
PUBLIC_IP=$(aws ec2 describe-instances --instance-ids i-09726de87ad1f9596 --region us-east-1 --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)

# Test the service
curl http://$PUBLIC_IP/health

# Access the web interface
open http://$PUBLIC_IP
```

## ⚙️ Configuration

### Application Configuration

The application uses a TOML configuration file located at `~/.config/dattavani-asr/dattavani-asr.toml`:

```toml
[aws]
region = "us-east-1"
s3_bucket = "dattavani"

[whisper]
model_size = "large-v3"
device = "auto"  # Will use GPU if available
compute_type = "float16"
task = "transcribe"

[processing]
max_workers = 2  # Adjust based on GPU memory
segment_duration = 300
target_sample_rate = 16000
chunk_size = 8192
timeout_seconds = 3600
retry_attempts = 3

[logging]
level = "info"
file = "/home/ubuntu/logs/dattavani-asr.log"
max_file_size = 10485760  # 10MB
max_files = 7

[storage]
temp_dir = "/tmp/dattavani_asr"
cache_dir = "/home/ubuntu/.cache/dattavani-asr"
output_prefix = "gen-transcript"
max_cache_size = 5368709120  # 5GB
```

### Environment Variables

```bash
# View current environment
./aws-deployment/deploy.sh ssh
cat ~/.env
```

### Service Configuration

The application runs as a systemd service:

```bash
# Service status
sudo systemctl status dattavani-asr

# Service logs
sudo journalctl -u dattavani-asr -f

# Restart service
sudo systemctl restart dattavani-asr
```

## 📊 Monitoring & Maintenance

### Health Monitoring

```bash
# Check application health
curl http://$PUBLIC_IP/health

# Monitor system resources
./aws-deployment/manage-infrastructure.sh monitor

# View application logs
./aws-deployment/deploy.sh logs
```

### Performance Monitoring

```bash
# SSH into instance
./aws-deployment/deploy.sh ssh

# Monitor GPU usage
nvidia-smi -l 1

# Monitor system resources
htop

# Monitor disk usage
df -h
du -sh ~/.cache/dattavani-asr/
```

### Log Management

```bash
# Application logs
tail -f ~/logs/dattavani-asr.log

# System service logs
sudo journalctl -u dattavani-asr -f

# Nginx logs
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log
```

### Backup and Recovery

```bash
# Create AMI backup
./aws-deployment/manage-infrastructure.sh backup

# List backups
./aws-deployment/manage-infrastructure.sh list-backups

# Clean old backups
./aws-deployment/manage-infrastructure.sh cleanup-backups
```

### Updates

```bash
# Update the application
cd dattavani-asr-rust
git pull origin main
./aws-deployment/deploy.sh deploy

# Or use the update script on the server
./aws-deployment/deploy.sh ssh
~/update-dattavani.sh
```

## 🔧 Troubleshooting

### Common Issues

1. **Service Won't Start**
   ```bash
   # Check service status
   sudo systemctl status dattavani-asr
   
   # Check logs
   sudo journalctl -u dattavani-asr -n 50
   
   # Test binary directly
   cd ~/projects/dattavani-asr-rust
   ./dattavani-asr health-check
   ```

2. **GPU Not Detected**
   ```bash
   # Check GPU status
   nvidia-smi
   
   # Install NVIDIA drivers if needed
   sudo apt update
   sudo apt install -y nvidia-driver-470
   sudo reboot
   ```

3. **S3 Access Issues**
   ```bash
   # Test AWS credentials
   aws sts get-caller-identity
   
   # Test S3 access
   aws s3 ls s3://dattavani/
   
   # Check IAM instance profile
   curl http://169.254.169.254/latest/meta-data/iam/security-credentials/
   ```

4. **High Memory Usage**
   ```bash
   # Check memory usage
   free -h
   
   # Reduce max_workers in configuration
   nano ~/.config/dattavani-asr/dattavani-asr.toml
   
   # Restart service
   sudo systemctl restart dattavani-asr
   ```

5. **Network Connectivity Issues**
   ```bash
   # Check security group
   ./aws-deployment/manage-infrastructure.sh status
   
   # Update security group
   ./aws-deployment/manage-infrastructure.sh update-sg
   
   # Test connectivity
   curl -I http://$PUBLIC_IP
   ```

### Debug Mode

```bash
# Enable debug logging
sudo systemctl edit dattavani-asr

# Add:
[Service]
Environment=RUST_LOG=debug

# Restart service
sudo systemctl restart dattavani-asr

# View debug logs
sudo journalctl -u dattavani-asr -f
```

### Performance Tuning

1. **GPU Memory Optimization**
   - Reduce `max_workers` if GPU memory is insufficient
   - Use smaller Whisper models for faster processing
   - Monitor GPU memory with `nvidia-smi`

2. **CPU Optimization**
   - Adjust `max_workers` based on CPU cores
   - Monitor CPU usage with `htop`

3. **Storage Optimization**
   - Clean temporary files regularly
   - Monitor disk usage
   - Adjust cache sizes

## 🔐 Security

### Security Best Practices

1. **Access Control**
   ```bash
   # Restrict SSH access to specific IPs
   aws ec2 authorize-security-group-ingress \
     --group-id sg-0264890af868ab040 \
     --protocol tcp \
     --port 22 \
     --cidr YOUR_IP/32
   
   # Remove broad SSH access
   aws ec2 revoke-security-group-ingress \
     --group-id sg-0264890af868ab040 \
     --protocol tcp \
     --port 22 \
     --cidr 0.0.0.0/0
   ```

2. **SSL/TLS Configuration**
   ```bash
   # Install Certbot for Let's Encrypt
   sudo apt install -y certbot python3-certbot-nginx
   
   # Get SSL certificate
   sudo certbot --nginx -d your-domain.com
   ```

3. **Firewall Configuration**
   ```bash
   # Enable UFW
   sudo ufw enable
   
   # Allow necessary ports
   sudo ufw allow 22/tcp
   sudo ufw allow 80/tcp
   sudo ufw allow 443/tcp
   ```

### Security Monitoring

```bash
# Setup CloudWatch alarms
./aws-deployment/manage-infrastructure.sh setup-alarms

# Monitor failed login attempts
sudo tail -f /var/log/auth.log

# Check for security updates
sudo apt list --upgradable
```

## 📈 Scaling

### Vertical Scaling

1. **Upgrade Instance Type**
   ```bash
   # Stop instance
   ./aws-deployment/manage-infrastructure.sh stop
   
   # Change instance type (via AWS Console or CLI)
   aws ec2 modify-instance-attribute \
     --instance-id i-09726de87ad1f9596 \
     --instance-type Value=g4ad.4xlarge
   
   # Start instance
   ./aws-deployment/manage-infrastructure.sh start
   ```

2. **Increase Storage**
   ```bash
   # Resize EBS volume (via AWS Console)
   # Then resize filesystem on instance:
   sudo resize2fs /dev/xvda1
   ```

### Horizontal Scaling

For high-demand scenarios, consider:

1. **Load Balancer with Multiple Instances**
2. **Auto Scaling Groups**
3. **Container Orchestration (ECS/EKS)**
4. **Serverless with AWS Lambda** (for smaller workloads)

### Container Deployment

```bash
# Use Docker Compose for local testing
cd aws-deployment
docker-compose up -d

# Or deploy to ECS
# (Requires additional ECS configuration)
```

## 💰 Cost Optimization

### Monitor Costs

```bash
# View estimated costs
./aws-deployment/manage-infrastructure.sh costs

# Set up billing alerts in AWS Console
```

### Cost Optimization Strategies

1. **Use Spot Instances** (for non-critical workloads)
2. **Schedule Instance Start/Stop**
   ```bash
   # Add to crontab for automatic shutdown
   0 22 * * * /usr/local/bin/aws ec2 stop-instances --instance-ids i-09726de87ad1f9596 --region us-east-1
   0 8 * * 1-5 /usr/local/bin/aws ec2 start-instances --instance-ids i-09726de87ad1f9596 --region us-east-1
   ```

3. **S3 Lifecycle Policies**
   ```bash
   # Move old files to cheaper storage classes
   aws s3api put-bucket-lifecycle-configuration \
     --bucket dattavani \
     --lifecycle-configuration file://lifecycle-policy.json
   ```

4. **Right-size Resources**
   - Monitor actual usage
   - Adjust instance type based on utilization
   - Optimize storage allocation

## 📞 Support and Maintenance

### Regular Maintenance Tasks

1. **Weekly**
   - Check service health
   - Review logs for errors
   - Monitor disk usage

2. **Monthly**
   - Create AMI backup
   - Update system packages
   - Review performance metrics

3. **Quarterly**
   - Security audit
   - Cost review
   - Performance optimization

### Emergency Procedures

1. **Service Down**
   ```bash
   # Quick restart
   sudo systemctl restart dattavani-asr
   
   # If that fails, restart instance
   ./aws-deployment/manage-infrastructure.sh restart
   ```

2. **High Resource Usage**
   ```bash
   # Check processes
   htop
   
   # Restart service
   sudo systemctl restart dattavani-asr
   
   # Scale up if needed
   ```

3. **Data Recovery**
   ```bash
   # Restore from AMI backup
   # (Use AWS Console to launch new instance from AMI)
   
   # Restore S3 data
   aws s3 sync s3://dattavani-backup/ s3://dattavani/
   ```

### Contact Information

For technical support:
- Check this documentation first
- Review application logs
- Contact the development team
- Create GitHub issues for bugs

---

**Deployment completed successfully! The Dattavani ASR Rust application is now running on AWS with GPU acceleration and production-ready configuration.**
