# Dattavani ASR Rust - Quick Deployment Guide

## 🚀 One-Command Deployment

```bash
# Deploy everything automatically
./aws-deployment/deploy.sh
```

## 📋 Prerequisites Checklist

- [ ] AWS CLI installed and configured
- [ ] SSH key for EC2 instance available
- [ ] Rust installed locally (for building)
- [ ] In the dattavani-asr-rust project directory

## 🏗️ Existing Infrastructure

| Resource | Value | Status |
|----------|-------|--------|
| **EC2 Instance** | `i-09726de87ad1f9596` | ✅ Available |
| **Instance Type** | `g4ad.2xlarge` (GPU) | ✅ Perfect for ASR |
| **S3 Bucket** | `dattavani` | ✅ Ready |
| **Security Group** | `sg-0264890af868ab040` | ✅ Configured |
| **Region** | `us-east-1` | ✅ Set |

## ⚡ Quick Commands

### Deployment
```bash
./aws-deployment/deploy.sh deploy    # Full deployment
./aws-deployment/deploy.sh start     # Start instance only
./aws-deployment/deploy.sh stop      # Stop instance
./aws-deployment/deploy.sh ssh       # SSH into instance
./aws-deployment/deploy.sh logs      # View logs
```

### Infrastructure Management
```bash
./aws-deployment/manage-infrastructure.sh status    # Show status
./aws-deployment/manage-infrastructure.sh start     # Start instance
./aws-deployment/manage-infrastructure.sh backup    # Create backup
./aws-deployment/manage-infrastructure.sh monitor   # View metrics
```

## 🔍 Verification Steps

1. **Check Instance Status**
   ```bash
   ./aws-deployment/manage-infrastructure.sh status
   ```

2. **Test Service Health**
   ```bash
   # Get public IP
   PUBLIC_IP=$(aws ec2 describe-instances --instance-ids i-09726de87ad1f9596 --region us-east-1 --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)
   
   # Test health endpoint
   curl http://$PUBLIC_IP/health
   ```

3. **Access Web Interface**
   ```bash
   echo "Service URL: http://$PUBLIC_IP"
   ```

## 🛠️ Configuration Files

| File | Purpose | Location |
|------|---------|----------|
| `dattavani-asr.toml` | App config | `~/.config/dattavani-asr/` |
| `.env` | Environment vars | `~/` |
| Service file | Systemd service | `/etc/systemd/system/` |
| Nginx config | Web server | `/etc/nginx/sites-available/` |

## 📊 Monitoring

### Service Status
```bash
sudo systemctl status dattavani-asr
```

### Logs
```bash
# Application logs
tail -f ~/logs/dattavani-asr.log

# Service logs
sudo journalctl -u dattavani-asr -f

# Nginx logs
sudo tail -f /var/log/nginx/access.log
```

### System Resources
```bash
# GPU usage
nvidia-smi

# System resources
htop

# Disk usage
df -h
```

## 🔧 Common Issues & Solutions

### Service Won't Start
```bash
# Check status
sudo systemctl status dattavani-asr

# Check logs
sudo journalctl -u dattavani-asr -n 20

# Test binary
cd ~/projects/dattavani-asr-rust
./dattavani-asr health-check
```

### Can't Connect
```bash
# Check security group
./aws-deployment/manage-infrastructure.sh update-sg

# Check instance state
./aws-deployment/manage-infrastructure.sh status
```

### High Resource Usage
```bash
# Reduce workers in config
nano ~/.config/dattavani-asr/dattavani-asr.toml
# Set max_workers = 1

# Restart service
sudo systemctl restart dattavani-asr
```

## 🔄 Update Process

```bash
# Pull latest changes
git pull origin main

# Redeploy
./aws-deployment/deploy.sh deploy
```

## 📱 Service URLs

After deployment, access these URLs:

- **Health Check**: `http://<PUBLIC_IP>/health`
- **Main Interface**: `http://<PUBLIC_IP>/`
- **API Docs**: `http://<PUBLIC_IP>/api/docs` (if available)

## 🆘 Emergency Commands

```bash
# Restart everything
sudo systemctl restart dattavani-asr nginx

# Reboot instance
./aws-deployment/manage-infrastructure.sh restart

# Stop instance (save costs)
./aws-deployment/manage-infrastructure.sh stop
```

## 📞 Support

1. Check logs first: `./aws-deployment/deploy.sh logs`
2. Review this guide and DEPLOYMENT.md
3. Check GitHub issues
4. Contact development team

---

**🎉 Your Dattavani ASR Rust application is ready for deployment on AWS!**

**Next Steps:**
1. Run `./aws-deployment/deploy.sh` to deploy
2. Test the service at the provided URL
3. Monitor logs and performance
4. Set up regular backups with `./aws-deployment/manage-infrastructure.sh backup`
