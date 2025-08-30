# Dattavani ASR Rust - AWS Deployment Guide

This guide covers deploying the Dattavani ASR Rust application on the existing AWS infrastructure.

## 🏗️ Existing Infrastructure

### EC2 Instance
- **Instance ID**: `i-09726de87ad1f9596`
- **Instance Type**: `g4ad.2xlarge` (4 vCPUs, 16GB RAM, AMD Radeon Pro V520 GPU)
- **Name**: `dattavani`
- **State**: Running (started automatically)
- **Private IP**: `172.31.39.55`
- **Availability Zone**: `us-east-1c`

### Storage
- **S3 Bucket**: `dattavani` (already exists)
- **EBS Volume**: `vol-02519e688a0f962b8` (attached to `/dev/sda1`)

### Security & Access
- **IAM Instance Profile**: `Dattavani-Instance-Profile`
- **Security Group**: `sg-0264890af868ab040` (launch-wizard-4)
  - SSH (22): Open to 0.0.0.0/0
  - Custom (7860): Open to 70.121.17.82/32
- **VPC**: `vpc-019cb60397dba00be`
- **Subnet**: `subnet-0edcdfda42c742ff8`

## 🚀 Deployment Steps

### 1. Connect to the Instance

```bash
# Get the public IP address
aws ec2 describe-instances --instance-ids i-09726de87ad1f9596 \
  --query 'Reservations[0].Instances[0].PublicIpAddress' --output text

# SSH into the instance
ssh -i ~/.ssh/your-key.pem ubuntu@<PUBLIC_IP>
```

### 2. System Setup

```bash
# Update system packages
sudo apt update && sudo apt upgrade -y

# Install essential packages
sudo apt install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ffmpeg \
    python3 \
    python3-pip \
    htop \
    tmux \
    unzip

# Install AWS CLI v2 (if not already installed)
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install
rm -rf aws awscliv2.zip

# Verify AWS CLI and instance profile
aws sts get-caller-identity
```

### 3. Install Rust

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version

# Install additional Rust components
rustup component add clippy rustfmt
```

### 4. Install Whisper

```bash
# Install OpenAI Whisper
pip3 install --user openai-whisper

# Add to PATH
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify installation
whisper --help

# Download Whisper models (optional - will download on first use)
python3 -c "import whisper; whisper.load_model('large-v3')"
```

### 5. Clone and Build the Project

```bash
# Create project directory
mkdir -p ~/projects
cd ~/projects

# Clone the project (assuming it's in a Git repository)
git clone <YOUR_REPOSITORY_URL> dattavani-asr-rust
cd dattavani-asr-rust

# Or copy from local machine using scp
# scp -i ~/.ssh/your-key.pem -r /path/to/dattavani-asr-rust ubuntu@<PUBLIC_IP>:~/projects/

# Build the project
cargo build --release

# Verify the build
./target/release/dattavani-asr --help
```

### 6. Configuration

```bash
# Create configuration directory
mkdir -p ~/.config/dattavani-asr

# Copy configuration template
cp dattavani-asr.toml.template ~/.config/dattavani-asr/dattavani-asr.toml

# Edit configuration
nano ~/.config/dattavani-asr/dattavani-asr.toml
```

**Configuration file (`~/.config/dattavani-asr/dattavani-asr.toml`):**

```toml
[aws]
region = "us-east-1"
s3_bucket = "dattavani"

[google]
# Add your Google Cloud credentials if needed
project_id = "your-project-id"
credentials_path = "/home/ubuntu/.config/gcloud/service-account-key.json"

[whisper]
model_size = "large-v3"
device = "cuda"  # Use GPU acceleration
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

### 7. Environment Setup

```bash
# Create environment file
cat > ~/.env << EOF
RUST_LOG=info
AWS_REGION=us-east-1
S3_BUCKET=dattavani
WHISPER_MODEL_SIZE=large-v3
MAX_WORKERS=2
TEMP_DIR=/tmp/dattavani_asr
CACHE_DIR=/home/ubuntu/.cache/dattavani-asr
CONFIG_PATH=/home/ubuntu/.config/dattavani-asr/dattavani-asr.toml
EOF

# Create necessary directories
mkdir -p ~/logs
mkdir -p ~/.cache/dattavani-asr
mkdir -p /tmp/dattavani_asr

# Set permissions
chmod 755 ~/logs ~/.cache/dattavani-asr /tmp/dattavani_asr
```

### 8. Test the Installation

```bash
# Test basic functionality
./target/release/dattavani-asr health-check

# Test S3 connectivity
aws s3 ls s3://dattavani/

# Test with a sample audio file (if available)
# ./target/release/dattavani-asr stream-process s3://dattavani/sample-audio.mp3
```

### 9. Create Systemd Service (Optional)

```bash
# Create systemd service file
sudo tee /etc/systemd/system/dattavani-asr.service > /dev/null << EOF
[Unit]
Description=Dattavani ASR Rust Service
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/projects/dattavani-asr-rust
ExecStart=/home/ubuntu/projects/dattavani-asr-rust/target/release/dattavani-asr server --port 7860
Restart=always
RestartSec=10
Environment=RUST_LOG=info
Environment=CONFIG_PATH=/home/ubuntu/.config/dattavani-asr/dattavani-asr.toml
EnvironmentFile=/home/ubuntu/.env

[Install]
WantedBy=multi-user.target
EOF

# Enable and start the service
sudo systemctl daemon-reload
sudo systemctl enable dattavani-asr
sudo systemctl start dattavani-asr

# Check service status
sudo systemctl status dattavani-asr
```

### 10. Configure Nginx (Optional - for web interface)

```bash
# Install Nginx
sudo apt install -y nginx

# Create Nginx configuration
sudo tee /etc/nginx/sites-available/dattavani-asr > /dev/null << EOF
server {
    listen 80;
    server_name _;

    location / {
        proxy_pass http://127.0.0.1:7860;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        
        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        
        # Increase timeouts for long-running transcriptions
        proxy_connect_timeout 300s;
        proxy_send_timeout 300s;
        proxy_read_timeout 300s;
    }
}
EOF

# Enable the site
sudo ln -s /etc/nginx/sites-available/dattavani-asr /etc/nginx/sites-enabled/
sudo rm -f /etc/nginx/sites-enabled/default

# Test and restart Nginx
sudo nginx -t
sudo systemctl restart nginx
sudo systemctl enable nginx
```

### 11. Update Security Group (if needed)

```bash
# Add HTTP access if using Nginx
aws ec2 authorize-security-group-ingress \
    --group-id sg-0264890af868ab040 \
    --protocol tcp \
    --port 80 \
    --cidr 0.0.0.0/0

# Add HTTPS access if using SSL
aws ec2 authorize-security-group-ingress \
    --group-id sg-0264890af868ab040 \
    --protocol tcp \
    --port 443 \
    --cidr 0.0.0.0/0
```

## 📊 Monitoring and Maintenance

### Log Management

```bash
# View application logs
tail -f ~/logs/dattavani-asr.log

# View system service logs
sudo journalctl -u dattavani-asr -f

# View Nginx logs
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log
```

### Performance Monitoring

```bash
# Monitor GPU usage
nvidia-smi -l 1

# Monitor system resources
htop

# Monitor disk usage
df -h
du -sh ~/.cache/dattavani-asr/
du -sh /tmp/dattavani_asr/
```

### Backup and Updates

```bash
# Create backup script
cat > ~/backup-dattavani.sh << 'EOF'
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/home/ubuntu/backups"

mkdir -p $BACKUP_DIR

# Backup configuration
tar -czf $BACKUP_DIR/config_$DATE.tar.gz \
    ~/.config/dattavani-asr/ \
    ~/.env \
    /etc/systemd/system/dattavani-asr.service \
    /etc/nginx/sites-available/dattavani-asr

# Backup logs (last 7 days)
find ~/logs/ -name "*.log*" -mtime -7 -exec tar -czf $BACKUP_DIR/logs_$DATE.tar.gz {} +

# Upload to S3
aws s3 cp $BACKUP_DIR/ s3://dattavani/backups/ --recursive

# Clean old local backups (keep last 5)
ls -t $BACKUP_DIR/*.tar.gz | tail -n +6 | xargs rm -f

echo "Backup completed: $DATE"
EOF

chmod +x ~/backup-dattavani.sh

# Add to crontab for daily backups
(crontab -l 2>/dev/null; echo "0 2 * * * /home/ubuntu/backup-dattavani.sh") | crontab -
```

### Update Script

```bash
# Create update script
cat > ~/update-dattavani.sh << 'EOF'
#!/bin/bash
set -e

echo "Starting Dattavani ASR update..."

cd ~/projects/dattavani-asr-rust

# Stop the service
sudo systemctl stop dattavani-asr

# Backup current binary
cp target/release/dattavani-asr target/release/dattavani-asr.backup

# Pull latest changes
git pull origin main

# Build new version
cargo build --release

# Test the new binary
if ./target/release/dattavani-asr health-check; then
    echo "Health check passed, starting service..."
    sudo systemctl start dattavani-asr
    echo "Update completed successfully!"
else
    echo "Health check failed, rolling back..."
    cp target/release/dattavani-asr.backup target/release/dattavani-asr
    sudo systemctl start dattavani-asr
    echo "Rollback completed"
    exit 1
fi
EOF

chmod +x ~/update-dattavani.sh
```

## 🔧 Troubleshooting

### Common Issues

1. **GPU not detected**:
   ```bash
   # Check GPU status
   nvidia-smi
   
   # Install NVIDIA drivers if needed
   sudo apt install -y nvidia-driver-470
   sudo reboot
   ```

2. **Whisper model download fails**:
   ```bash
   # Manually download models
   python3 -c "import whisper; whisper.load_model('large-v3')"
   ```

3. **S3 access denied**:
   ```bash
   # Check IAM instance profile
   aws sts get-caller-identity
   
   # Test S3 access
   aws s3 ls s3://dattavani/
   ```

4. **Service won't start**:
   ```bash
   # Check service logs
   sudo journalctl -u dattavani-asr -n 50
   
   # Check configuration
   ./target/release/dattavani-asr health-check
   ```

### Performance Optimization

1. **GPU Memory Optimization**:
   - Reduce `max_workers` if GPU memory is insufficient
   - Use smaller Whisper models for faster processing
   - Monitor GPU memory usage with `nvidia-smi`

2. **Storage Optimization**:
   - Regularly clean temporary files
   - Set up log rotation
   - Monitor disk usage

3. **Network Optimization**:
   - Use CloudFront for static assets
   - Enable gzip compression in Nginx
   - Optimize S3 transfer settings

## 📈 Scaling Considerations

### Horizontal Scaling
- Use Application Load Balancer with multiple instances
- Implement job queuing with SQS
- Use Auto Scaling Groups for demand-based scaling

### Vertical Scaling
- Upgrade to larger GPU instances (p3, p4, g5 series)
- Increase EBS volume size and IOPS
- Optimize memory allocation

### Cost Optimization
- Use Spot Instances for batch processing
- Implement intelligent caching
- Set up lifecycle policies for S3 storage

## 🔐 Security Best Practices

1. **Access Control**:
   - Restrict security group rules to specific IPs
   - Use IAM roles with minimal permissions
   - Enable CloudTrail for audit logging

2. **Data Protection**:
   - Enable S3 encryption at rest
   - Use HTTPS for all communications
   - Implement data retention policies

3. **Monitoring**:
   - Set up CloudWatch alarms
   - Enable VPC Flow Logs
   - Monitor application metrics

## 📞 Support

For issues and questions:
- Check the troubleshooting section above
- Review application logs
- Monitor system resources
- Contact the development team

---

**Deployment completed on existing AWS infrastructure with GPU acceleration enabled.**
