#!/bin/bash

# Dattavani ASR Rust - Infrastructure Management Script
# This script manages the AWS infrastructure for the dattavani-asr-rust project

set -e

# Configuration
INSTANCE_ID="i-09726de87ad1f9596"
REGION="us-east-1"
S3_BUCKET="dattavani"
SECURITY_GROUP_ID="sg-0264890af868ab040"
PROJECT_NAME="dattavani-asr-rust"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
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

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}"
}

# Check AWS CLI and credentials
check_aws() {
    if ! command -v aws &> /dev/null; then
        error "AWS CLI is not installed"
    fi
    
    if ! aws sts get-caller-identity &> /dev/null; then
        error "AWS credentials not configured"
    fi
    
    log "AWS CLI and credentials verified ✓"
}

# Show infrastructure status
show_status() {
    log "Dattavani ASR Infrastructure Status"
    echo "===================================="
    
    # EC2 Instance Status
    echo -e "\n${BLUE}EC2 Instance:${NC}"
    local instance_info=$(aws ec2 describe-instances \
        --instance-ids $INSTANCE_ID \
        --region $REGION \
        --query 'Reservations[0].Instances[0].[InstanceId,InstanceType,State.Name,PublicIpAddress,PrivateIpAddress]' \
        --output table)
    echo "$instance_info"
    
    # S3 Bucket Status
    echo -e "\n${BLUE}S3 Bucket:${NC}"
    if aws s3api head-bucket --bucket $S3_BUCKET 2>/dev/null; then
        local bucket_size=$(aws s3 ls s3://$S3_BUCKET --recursive --summarize | grep "Total Size" | awk '{print $3, $4}')
        local object_count=$(aws s3 ls s3://$S3_BUCKET --recursive --summarize | grep "Total Objects" | awk '{print $3}')
        echo "Bucket: $S3_BUCKET"
        echo "Size: ${bucket_size:-0 Bytes}"
        echo "Objects: ${object_count:-0}"
    else
        warn "S3 bucket $S3_BUCKET not accessible"
    fi
    
    # Security Group Status
    echo -e "\n${BLUE}Security Group:${NC}"
    aws ec2 describe-security-groups \
        --group-ids $SECURITY_GROUP_ID \
        --region $REGION \
        --query 'SecurityGroups[0].[GroupId,GroupName,Description]' \
        --output table
    
    # Security Group Rules
    echo -e "\n${BLUE}Security Group Rules:${NC}"
    echo "Inbound Rules:"
    aws ec2 describe-security-groups \
        --group-ids $SECURITY_GROUP_ID \
        --region $REGION \
        --query 'SecurityGroups[0].IpPermissions[*].[IpProtocol,FromPort,ToPort,IpRanges[0].CidrIp]' \
        --output table
}

# Start EC2 instance
start_instance() {
    log "Starting EC2 instance $INSTANCE_ID..."
    
    local current_state=$(aws ec2 describe-instances \
        --instance-ids $INSTANCE_ID \
        --region $REGION \
        --query 'Reservations[0].Instances[0].State.Name' \
        --output text)
    
    if [ "$current_state" = "running" ]; then
        log "Instance is already running"
        return 0
    fi
    
    aws ec2 start-instances --instance-ids $INSTANCE_ID --region $REGION
    
    log "Waiting for instance to be running..."
    aws ec2 wait instance-running --instance-ids $INSTANCE_ID --region $REGION
    
    local public_ip=$(aws ec2 describe-instances \
        --instance-ids $INSTANCE_ID \
        --region $REGION \
        --query 'Reservations[0].Instances[0].PublicIpAddress' \
        --output text)
    
    log "Instance started successfully. Public IP: $public_ip"
}

# Stop EC2 instance
stop_instance() {
    log "Stopping EC2 instance $INSTANCE_ID..."
    
    local current_state=$(aws ec2 describe-instances \
        --instance-ids $INSTANCE_ID \
        --region $REGION \
        --query 'Reservations[0].Instances[0].State.Name' \
        --output text)
    
    if [ "$current_state" = "stopped" ]; then
        log "Instance is already stopped"
        return 0
    fi
    
    aws ec2 stop-instances --instance-ids $INSTANCE_ID --region $REGION
    
    log "Waiting for instance to be stopped..."
    aws ec2 wait instance-stopped --instance-ids $INSTANCE_ID --region $REGION
    
    log "Instance stopped successfully"
}

# Restart EC2 instance
restart_instance() {
    log "Restarting EC2 instance $INSTANCE_ID..."
    
    aws ec2 reboot-instances --instance-ids $INSTANCE_ID --region $REGION
    
    log "Instance restart initiated. Waiting for it to be running..."
    sleep 10
    aws ec2 wait instance-running --instance-ids $INSTANCE_ID --region $REGION
    
    log "Instance restarted successfully"
}

# Create AMI backup
create_backup() {
    log "Creating AMI backup of instance $INSTANCE_ID..."
    
    local timestamp=$(date +%Y%m%d-%H%M%S)
    local ami_name="dattavani-asr-backup-$timestamp"
    
    local ami_id=$(aws ec2 create-image \
        --instance-id $INSTANCE_ID \
        --name "$ami_name" \
        --description "Backup of Dattavani ASR instance created on $timestamp" \
        --region $REGION \
        --query 'ImageId' \
        --output text)
    
    log "AMI backup created: $ami_id"
    log "AMI Name: $ami_name"
    
    # Tag the AMI
    aws ec2 create-tags \
        --resources $ami_id \
        --tags Key=Name,Value="$ami_name" \
               Key=Project,Value="$PROJECT_NAME" \
               Key=Type,Value="Backup" \
               Key=CreatedDate,Value="$timestamp" \
        --region $REGION
    
    log "AMI backup tagged successfully"
}

# List AMI backups
list_backups() {
    log "Listing AMI backups for Dattavani ASR..."
    
    aws ec2 describe-images \
        --owners self \
        --filters "Name=name,Values=dattavani-asr-backup-*" \
        --region $REGION \
        --query 'Images[*].[ImageId,Name,CreationDate,State]' \
        --output table
}

# Clean old backups (keep last 5)
cleanup_backups() {
    log "Cleaning up old AMI backups (keeping last 5)..."
    
    local backup_amis=$(aws ec2 describe-images \
        --owners self \
        --filters "Name=name,Values=dattavani-asr-backup-*" \
        --region $REGION \
        --query 'Images[*].[ImageId,CreationDate]' \
        --output text | sort -k2 | head -n -5 | awk '{print $1}')
    
    if [ -z "$backup_amis" ]; then
        log "No old backups to clean up"
        return 0
    fi
    
    for ami_id in $backup_amis; do
        log "Deregistering AMI: $ami_id"
        aws ec2 deregister-image --image-id $ami_id --region $REGION
        
        # Get associated snapshots
        local snapshots=$(aws ec2 describe-images \
            --image-ids $ami_id \
            --region $REGION \
            --query 'Images[0].BlockDeviceMappings[*].Ebs.SnapshotId' \
            --output text 2>/dev/null || echo "")
        
        for snapshot_id in $snapshots; do
            if [ "$snapshot_id" != "None" ] && [ -n "$snapshot_id" ]; then
                log "Deleting snapshot: $snapshot_id"
                aws ec2 delete-snapshot --snapshot-id $snapshot_id --region $REGION
            fi
        done
    done
    
    log "Backup cleanup completed"
}

# Monitor instance metrics
monitor_instance() {
    log "Monitoring instance metrics for the last hour..."
    
    local end_time=$(date -u +%Y-%m-%dT%H:%M:%S)
    local start_time=$(date -u -d '1 hour ago' +%Y-%m-%dT%H:%M:%S)
    
    echo -e "\n${BLUE}CPU Utilization:${NC}"
    aws cloudwatch get-metric-statistics \
        --namespace AWS/EC2 \
        --metric-name CPUUtilization \
        --dimensions Name=InstanceId,Value=$INSTANCE_ID \
        --start-time $start_time \
        --end-time $end_time \
        --period 300 \
        --statistics Average \
        --region $REGION \
        --query 'Datapoints[*].[Timestamp,Average]' \
        --output table
    
    echo -e "\n${BLUE}Network In (Bytes):${NC}"
    aws cloudwatch get-metric-statistics \
        --namespace AWS/EC2 \
        --metric-name NetworkIn \
        --dimensions Name=InstanceId,Value=$INSTANCE_ID \
        --start-time $start_time \
        --end-time $end_time \
        --period 300 \
        --statistics Sum \
        --region $REGION \
        --query 'Datapoints[*].[Timestamp,Sum]' \
        --output table
}

# Update security group rules
update_security_group() {
    log "Updating security group rules..."
    
    # Check current rules
    local current_rules=$(aws ec2 describe-security-groups \
        --group-ids $SECURITY_GROUP_ID \
        --region $REGION \
        --query 'SecurityGroups[0].IpPermissions[*].FromPort' \
        --output text)
    
    # Add HTTP (80) if not present
    if [[ ! $current_rules =~ "80" ]]; then
        log "Adding HTTP (port 80) rule..."
        aws ec2 authorize-security-group-ingress \
            --group-id $SECURITY_GROUP_ID \
            --protocol tcp \
            --port 80 \
            --cidr 0.0.0.0/0 \
            --region $REGION || warn "HTTP rule might already exist"
    fi
    
    # Add HTTPS (443) if not present
    if [[ ! $current_rules =~ "443" ]]; then
        log "Adding HTTPS (port 443) rule..."
        aws ec2 authorize-security-group-ingress \
            --group-id $SECURITY_GROUP_ID \
            --protocol tcp \
            --port 443 \
            --cidr 0.0.0.0/0 \
            --region $REGION || warn "HTTPS rule might already exist"
    fi
    
    log "Security group rules updated"
}

# Check S3 bucket contents
check_s3_bucket() {
    log "Checking S3 bucket contents..."
    
    echo -e "\n${BLUE}Recent files in $S3_BUCKET:${NC}"
    aws s3 ls s3://$S3_BUCKET/ --recursive --human-readable | tail -20
    
    echo -e "\n${BLUE}Bucket summary:${NC}"
    aws s3 ls s3://$S3_BUCKET --recursive --summarize
}

# Setup CloudWatch alarms
setup_cloudwatch_alarms() {
    log "Setting up CloudWatch alarms..."
    
    # High CPU alarm
    aws cloudwatch put-metric-alarm \
        --alarm-name "dattavani-asr-high-cpu" \
        --alarm-description "Alarm when CPU exceeds 80%" \
        --metric-name CPUUtilization \
        --namespace AWS/EC2 \
        --statistic Average \
        --period 300 \
        --threshold 80 \
        --comparison-operator GreaterThanThreshold \
        --evaluation-periods 2 \
        --dimensions Name=InstanceId,Value=$INSTANCE_ID \
        --region $REGION
    
    # Instance status check alarm
    aws cloudwatch put-metric-alarm \
        --alarm-name "dattavani-asr-status-check" \
        --alarm-description "Alarm when instance status check fails" \
        --metric-name StatusCheckFailed \
        --namespace AWS/EC2 \
        --statistic Maximum \
        --period 60 \
        --threshold 0 \
        --comparison-operator GreaterThanThreshold \
        --evaluation-periods 2 \
        --dimensions Name=InstanceId,Value=$INSTANCE_ID \
        --region $REGION
    
    log "CloudWatch alarms created successfully"
}

# Show costs
show_costs() {
    log "Showing estimated costs for the last 30 days..."
    
    local end_date=$(date +%Y-%m-%d)
    local start_date=$(date -d '30 days ago' +%Y-%m-%d)
    
    aws ce get-cost-and-usage \
        --time-period Start=$start_date,End=$end_date \
        --granularity MONTHLY \
        --metrics BlendedCost \
        --group-by Type=DIMENSION,Key=SERVICE \
        --region us-east-1 \
        --query 'ResultsByTime[0].Groups[*].[Keys[0],Metrics.BlendedCost.Amount]' \
        --output table
}

# SSH into instance
ssh_instance() {
    local public_ip=$(aws ec2 describe-instances \
        --instance-ids $INSTANCE_ID \
        --region $REGION \
        --query 'Reservations[0].Instances[0].PublicIpAddress' \
        --output text)
    
    if [ "$public_ip" = "None" ] || [ -z "$public_ip" ]; then
        error "Instance does not have a public IP address"
    fi
    
    log "Connecting to instance at $public_ip..."
    ssh -o StrictHostKeyChecking=no ubuntu@$public_ip
}

# Show help
show_help() {
    echo "Dattavani ASR Infrastructure Management Script"
    echo "=============================================="
    echo ""
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  status              - Show infrastructure status"
    echo "  start               - Start EC2 instance"
    echo "  stop                - Stop EC2 instance"
    echo "  restart             - Restart EC2 instance"
    echo "  ssh                 - SSH into the instance"
    echo "  backup              - Create AMI backup"
    echo "  list-backups        - List AMI backups"
    echo "  cleanup-backups     - Clean old backups (keep last 5)"
    echo "  monitor             - Show instance metrics"
    echo "  update-sg           - Update security group rules"
    echo "  check-s3            - Check S3 bucket contents"
    echo "  setup-alarms        - Setup CloudWatch alarms"
    echo "  costs               - Show estimated costs"
    echo "  help                - Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 status           # Show current infrastructure status"
    echo "  $0 start            # Start the EC2 instance"
    echo "  $0 backup           # Create a backup AMI"
    echo "  $0 monitor          # View performance metrics"
}

# Main function
main() {
    check_aws
    
    case "${1:-status}" in
        "status")
            show_status
            ;;
        "start")
            start_instance
            ;;
        "stop")
            stop_instance
            ;;
        "restart")
            restart_instance
            ;;
        "ssh")
            ssh_instance
            ;;
        "backup")
            create_backup
            ;;
        "list-backups")
            list_backups
            ;;
        "cleanup-backups")
            cleanup_backups
            ;;
        "monitor")
            monitor_instance
            ;;
        "update-sg")
            update_security_group
            ;;
        "check-s3")
            check_s3_bucket
            ;;
        "setup-alarms")
            setup_cloudwatch_alarms
            ;;
        "costs")
            show_costs
            ;;
        "help")
            show_help
            ;;
        *)
            error "Unknown command: $1. Use '$0 help' for usage information."
            ;;
    esac
}

# Run main function
main "$@"
