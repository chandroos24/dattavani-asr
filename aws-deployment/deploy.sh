#!/bin/bash

# Dattavani ASR Rust - Automated Deployment Script
# This script deploys the application to the existing dattavani EC2 instance

set -e

# Configuration
INSTANCE_ID="i-09726de87ad1f9596"
REGION="us-east-1"
PROJECT_NAME="dattavani-asr-rust"
S3_BUCKET="dattavani"
REMOTE_USER="ubuntu"
REMOTE_DIR="/home/ubuntu/projects"
SERVICE_PORT="7860"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check AWS CLI
    if ! command -v aws &> /dev/null; then
        error "AWS CLI is not installed. Please install it first."
    fi
    
    # Check AWS credentials
    if ! aws sts get-caller-identity &> /dev/null; then
        error "AWS credentials not configured. Please run 'aws configure'."
    fi
    
    # Check if we can access the instance
    if ! aws ec2 describe-instances --instance-ids $INSTANCE_ID --region $REGION &> /dev/null; then
        error "Cannot access instance $INSTANCE_ID. Check your permissions."
    fi
    
    # Check if Rust project exists
    if [ ! -f "Cargo.toml" ]; then
        error "This script must be run from the dattavani-asr-rust project root directory."
    fi
    
    log "Prerequisites check passed ✓"
}

# Start the EC2 instance if it's stopped
start_instance() {
    log "Checking instance state..."
    
    local state=$(aws ec2 describe-instances \
        --instance-ids $INSTANCE_ID \
        --region $REGION \
        --query 'Reservations[0].Instances[0].State.Name' \
        --output text)
    
    if [ "$state" = "stopped" ]; then
        log "Starting instance $INSTANCE_ID..."
        aws ec2 start-instances --instance-ids $INSTANCE_ID --region $REGION
        
        log "Waiting for instance to be running..."
        aws ec2 wait instance-running --instance-ids $INSTANCE_ID --region $REGION
        
        # Wait additional time for SSH to be ready
        log "Waiting for SSH to be ready..."
        sleep 30
    elif [ "$state" = "running" ]; then
        log "Instance is already running ✓"
    else
        error "Instance is in state: $state. Cannot proceed."
    fi
}

# Get instance public IP
get_public_ip() {
    PUBLIC_IP=$(aws ec2 describe-instances \
        --instance-ids $INSTANCE_ID \
        --region $REGION \
        --query 'Reservations[0].Instances[0].PublicIpAddress' \
        --output text)
    
    if [ "$PUBLIC_IP" = "None" ] || [ -z "$PUBLIC_IP" ]; then
        error "Could not get public IP for instance $INSTANCE_ID"
    fi
    
    log "Instance public IP: $PUBLIC_IP"
}

# Test SSH connectivity
test_ssh() {
    log "Testing SSH connectivity..."
    
    local max_attempts=10
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP "echo 'SSH connection successful'" &> /dev/null; then
            log "SSH connection successful ✓"
            return 0
        fi
        
        warn "SSH attempt $attempt/$max_attempts failed. Retrying in 10 seconds..."
        sleep 10
        ((attempt++))
    done
    
    error "Could not establish SSH connection after $max_attempts attempts"
}

# Build the project locally
build_project() {
    log "Building project locally..."
    
    # Clean previous builds
    cargo clean
    
    # Build release version
    cargo build --release
    
    if [ ! -f "target/release/dattavani-asr" ]; then
        error "Build failed - binary not found"
    fi
    
    log "Build completed successfully ✓"
}

# Create deployment package
create_deployment_package() {
    log "Creating deployment package..."
    
    local temp_dir=$(mktemp -d)
    local package_dir="$temp_dir/$PROJECT_NAME"
    
    mkdir -p "$package_dir"
    
    # Copy binary
    cp target/release/dattavani-asr "$package_dir/"
    
    # Copy configuration files
    cp -r aws-deployment "$package_dir/"
    
    # Copy other necessary files
    [ -f "dattavani-asr.toml.template" ] && cp dattavani-asr.toml.template "$package_dir/"
    [ -f ".env.template" ] && cp .env.template "$package_dir/"
    [ -f "README.md" ] && cp README.md "$package_dir/"
    
    # Create archive
    cd "$temp_dir"
    tar -czf "$PROJECT_NAME-deployment.tar.gz" "$PROJECT_NAME"
    
    # Move to current directory
    mv "$PROJECT_NAME-deployment.tar.gz" "$OLDPWD/"
    
    # Cleanup
    rm -rf "$temp_dir"
    
    log "Deployment package created: $PROJECT_NAME-deployment.tar.gz ✓"
}

# Upload and extract on remote server
upload_and_extract() {
    log "Uploading deployment package to server..."
    
    # Upload package
    scp -o StrictHostKeyChecking=no "$PROJECT_NAME-deployment.tar.gz" $REMOTE_USER@$PUBLIC_IP:/tmp/
    
    # Extract and setup on remote server
    ssh -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP << EOF
        set -e
        
        # Create project directory
        mkdir -p $REMOTE_DIR
        cd $REMOTE_DIR
        
        # Backup existing installation if it exists
        if [ -d "$PROJECT_NAME" ]; then
            echo "Backing up existing installation..."
            mv $PROJECT_NAME ${PROJECT_NAME}_backup_\$(date +%Y%m%d_%H%M%S)
        fi
        
        # Extract new version
        tar -xzf /tmp/$PROJECT_NAME-deployment.tar.gz
        
        # Set permissions
        chmod +x $PROJECT_NAME/dattavani-asr
        
        # Cleanup
        rm -f /tmp/$PROJECT_NAME-deployment.tar.gz
        
        echo "Deployment package extracted successfully"
EOF
    
    log "Upload and extraction completed ✓"
}

# Setup system dependencies
setup_dependencies() {
    log "Setting up system dependencies..."
    
    ssh -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP << 'EOF'
        set -e
        
        # Update system
        sudo apt update
        
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
            unzip \
            nginx
        
        # Install Rust if not present
        if ! command -v cargo &> /dev/null; then
            echo "Installing Rust..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source ~/.cargo/env
        fi
        
        # Install Whisper if not present
        if ! command -v whisper &> /dev/null; then
            echo "Installing OpenAI Whisper..."
            pip3 install --user openai-whisper
            echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
        fi
        
        echo "Dependencies setup completed"
EOF
    
    log "System dependencies setup completed ✓"
}

# Configure the application
configure_application() {
    log "Configuring application..."
    
    ssh -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP << EOF
        set -e
        
        cd $REMOTE_DIR/$PROJECT_NAME
        
        # Create configuration directory
        mkdir -p ~/.config/dattavani-asr
        mkdir -p ~/logs
        mkdir -p ~/.cache/dattavani-asr
        mkdir -p /tmp/dattavani_asr
        
        # Create configuration file
        cat > ~/.config/dattavani-asr/dattavani-asr.toml << 'TOML_EOF'
[aws]
region = "$REGION"
s3_bucket = "$S3_BUCKET"

[whisper]
model_size = "large-v3"
device = "auto"
compute_type = "float16"
task = "transcribe"

[processing]
max_workers = 2
segment_duration = 300
target_sample_rate = 16000
chunk_size = 8192
timeout_seconds = 3600
retry_attempts = 3

[logging]
level = "info"
file = "/home/ubuntu/logs/dattavani-asr.log"
max_file_size = 10485760
max_files = 7

[storage]
temp_dir = "/tmp/dattavani_asr"
cache_dir = "/home/ubuntu/.cache/dattavani-asr"
output_prefix = "gen-transcript"
max_cache_size = 5368709120
TOML_EOF
        
        # Create environment file
        cat > ~/.env << 'ENV_EOF'
RUST_LOG=info
AWS_REGION=$REGION
S3_BUCKET=$S3_BUCKET
WHISPER_MODEL_SIZE=large-v3
MAX_WORKERS=2
TEMP_DIR=/tmp/dattavani_asr
CACHE_DIR=/home/ubuntu/.cache/dattavani-asr
CONFIG_PATH=/home/ubuntu/.config/dattavani-asr/dattavani-asr.toml
ENV_EOF
        
        echo "Application configuration completed"
EOF
    
    log "Application configuration completed ✓"
}

# Setup systemd service
setup_service() {
    log "Setting up systemd service..."
    
    ssh -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP << EOF
        set -e
        
        # Create systemd service file
        sudo tee /etc/systemd/system/dattavani-asr.service > /dev/null << 'SERVICE_EOF'
[Unit]
Description=Dattavani ASR Rust Service
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=$REMOTE_DIR/$PROJECT_NAME
ExecStart=$REMOTE_DIR/$PROJECT_NAME/dattavani-asr server --port $SERVICE_PORT
Restart=always
RestartSec=10
Environment=RUST_LOG=info
Environment=CONFIG_PATH=/home/ubuntu/.config/dattavani-asr/dattavani-asr.toml
EnvironmentFile=/home/ubuntu/.env

[Install]
WantedBy=multi-user.target
SERVICE_EOF
        
        # Reload systemd and enable service
        sudo systemctl daemon-reload
        sudo systemctl enable dattavani-asr
        
        echo "Systemd service setup completed"
EOF
    
    log "Systemd service setup completed ✓"
}

# Configure Nginx
configure_nginx() {
    log "Configuring Nginx..."
    
    ssh -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP << EOF
        set -e
        
        # Create Nginx configuration
        sudo tee /etc/nginx/sites-available/dattavani-asr > /dev/null << 'NGINX_EOF'
server {
    listen 80;
    server_name _;

    location / {
        proxy_pass http://127.0.0.1:$SERVICE_PORT;
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
        
        # Increase body size for file uploads
        client_max_body_size 100M;
    }
}
NGINX_EOF
        
        # Enable the site
        sudo ln -sf /etc/nginx/sites-available/dattavani-asr /etc/nginx/sites-enabled/
        sudo rm -f /etc/nginx/sites-enabled/default
        
        # Test and restart Nginx
        sudo nginx -t
        sudo systemctl restart nginx
        sudo systemctl enable nginx
        
        echo "Nginx configuration completed"
EOF
    
    log "Nginx configuration completed ✓"
}

# Test the deployment
test_deployment() {
    log "Testing deployment..."
    
    ssh -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP << EOF
        set -e
        
        cd $REMOTE_DIR/$PROJECT_NAME
        
        # Test binary
        ./dattavani-asr --version
        
        # Test health check
        ./dattavani-asr health-check
        
        # Test AWS connectivity
        aws sts get-caller-identity
        
        # Test S3 access
        aws s3 ls s3://$S3_BUCKET/ || echo "S3 bucket is empty or access denied"
        
        echo "Deployment tests completed"
EOF
    
    log "Deployment tests completed ✓"
}

# Start the service
start_service() {
    log "Starting Dattavani ASR service..."
    
    ssh -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP << 'EOF'
        set -e
        
        # Start the service
        sudo systemctl start dattavani-asr
        
        # Wait a moment for service to start
        sleep 5
        
        # Check service status
        sudo systemctl status dattavani-asr --no-pager
        
        echo "Service started successfully"
EOF
    
    log "Service started successfully ✓"
}

# Update security group if needed
update_security_group() {
    log "Checking security group configuration..."
    
    # Check if HTTP port is already open
    local http_open=$(aws ec2 describe-security-groups \
        --group-ids sg-0264890af868ab040 \
        --region $REGION \
        --query 'SecurityGroups[0].IpPermissions[?FromPort==`80`]' \
        --output text)
    
    if [ -z "$http_open" ]; then
        log "Opening HTTP port 80 in security group..."
        aws ec2 authorize-security-group-ingress \
            --group-id sg-0264890af868ab040 \
            --protocol tcp \
            --port 80 \
            --cidr 0.0.0.0/0 \
            --region $REGION || warn "HTTP port might already be open"
    else
        log "HTTP port 80 is already open ✓"
    fi
}

# Display deployment summary
display_summary() {
    log "Deployment Summary"
    echo "===================="
    echo "Instance ID: $INSTANCE_ID"
    echo "Public IP: $PUBLIC_IP"
    echo "Service URL: http://$PUBLIC_IP"
    echo "SSH Access: ssh $REMOTE_USER@$PUBLIC_IP"
    echo "Service Status: sudo systemctl status dattavani-asr"
    echo "Logs: tail -f ~/logs/dattavani-asr.log"
    echo "===================="
    
    log "Deployment completed successfully! 🎉"
    log "You can now access the Dattavani ASR service at: http://$PUBLIC_IP"
}

# Cleanup function
cleanup() {
    log "Cleaning up temporary files..."
    rm -f "$PROJECT_NAME-deployment.tar.gz"
}

# Main deployment function
main() {
    log "Starting Dattavani ASR Rust deployment..."
    
    # Set trap for cleanup
    trap cleanup EXIT
    
    # Run deployment steps
    check_prerequisites
    start_instance
    get_public_ip
    test_ssh
    build_project
    create_deployment_package
    upload_and_extract
    setup_dependencies
    configure_application
    setup_service
    configure_nginx
    update_security_group
    test_deployment
    start_service
    display_summary
}

# Parse command line arguments
case "${1:-deploy}" in
    "deploy")
        main
        ;;
    "start")
        start_instance
        get_public_ip
        log "Instance started. Public IP: $PUBLIC_IP"
        ;;
    "stop")
        log "Stopping instance $INSTANCE_ID..."
        aws ec2 stop-instances --instance-ids $INSTANCE_ID --region $REGION
        log "Instance stop initiated"
        ;;
    "status")
        get_public_ip
        log "Instance Status:"
        aws ec2 describe-instances --instance-ids $INSTANCE_ID --region $REGION \
            --query 'Reservations[0].Instances[0].State.Name' --output text
        log "Public IP: $PUBLIC_IP"
        ;;
    "ssh")
        get_public_ip
        log "Connecting to $PUBLIC_IP..."
        ssh -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP
        ;;
    "logs")
        get_public_ip
        log "Viewing service logs..."
        ssh -o StrictHostKeyChecking=no $REMOTE_USER@$PUBLIC_IP "tail -f ~/logs/dattavani-asr.log"
        ;;
    "help")
        echo "Usage: $0 [command]"
        echo "Commands:"
        echo "  deploy  - Full deployment (default)"
        echo "  start   - Start the EC2 instance"
        echo "  stop    - Stop the EC2 instance"
        echo "  status  - Show instance status"
        echo "  ssh     - SSH into the instance"
        echo "  logs    - View application logs"
        echo "  help    - Show this help"
        ;;
    *)
        error "Unknown command: $1. Use '$0 help' for usage information."
        ;;
esac
