# GitHub Secrets Setup Guide

This guide explains how to set up the required GitHub repository secrets for automated CI/CD deployment.

## 🔐 Required Secrets

You need to configure the following secrets in your GitHub repository:

### 1. AWS Credentials

#### `AWS_ACCESS_KEY_ID`
- **Description**: AWS Access Key ID for deployment
- **Value**: Your AWS access key ID
- **How to get**: 
  ```bash
  aws configure list
  # Or create new IAM user with deployment permissions
  ```

#### `AWS_SECRET_ACCESS_KEY`
- **Description**: AWS Secret Access Key for deployment
- **Value**: Your AWS secret access key
- **How to get**: Same as above, keep this secure!

### 2. SSH Access

#### `EC2_SSH_PRIVATE_KEY`
- **Description**: Private SSH key for EC2 instance access
- **Value**: Contents of your private key file (e.g., `~/.ssh/your-key.pem`)
- **Format**: 
  ```
  -----BEGIN RSA PRIVATE KEY-----
  [Your private key content here]
  -----END RSA PRIVATE KEY-----
  ```

## 🛠️ How to Add Secrets

### Method 1: GitHub Web Interface

1. Go to your repository on GitHub
2. Click **Settings** tab
3. In the left sidebar, click **Secrets and variables** → **Actions**
4. Click **New repository secret**
5. Add each secret with the name and value specified above

### Method 2: GitHub CLI

```bash
# Install GitHub CLI if not already installed
# macOS: brew install gh
# Linux: See https://cli.github.com/

# Login to GitHub
gh auth login

# Add secrets (you'll be prompted for values)
gh secret set AWS_ACCESS_KEY_ID
gh secret set AWS_SECRET_ACCESS_KEY
gh secret set EC2_SSH_PRIVATE_KEY
```

### Method 3: Using Scripts

```bash
# Create a script to add all secrets
cat > setup-secrets.sh << 'EOF'
#!/bin/bash

echo "Setting up GitHub secrets for dattavani-asr-rust..."

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    echo "GitHub CLI is not installed. Please install it first."
    exit 1
fi

# Check if user is logged in
if ! gh auth status &> /dev/null; then
    echo "Please login to GitHub CLI first: gh auth login"
    exit 1
fi

# AWS Access Key ID
echo "Enter AWS Access Key ID:"
read -r AWS_ACCESS_KEY_ID
gh secret set AWS_ACCESS_KEY_ID --body "$AWS_ACCESS_KEY_ID"

# AWS Secret Access Key
echo "Enter AWS Secret Access Key:"
read -rs AWS_SECRET_ACCESS_KEY
gh secret set AWS_SECRET_ACCESS_KEY --body "$AWS_SECRET_ACCESS_KEY"

# SSH Private Key
echo "Enter path to SSH private key file (e.g., ~/.ssh/your-key.pem):"
read -r SSH_KEY_PATH
if [ -f "$SSH_KEY_PATH" ]; then
    gh secret set EC2_SSH_PRIVATE_KEY < "$SSH_KEY_PATH"
else
    echo "SSH key file not found: $SSH_KEY_PATH"
    exit 1
fi

echo "✅ All secrets have been set up successfully!"
EOF

chmod +x setup-secrets.sh
./setup-secrets.sh
```

## 🔍 Verify Secrets

After adding secrets, verify they're set correctly:

```bash
# List all secrets (values won't be shown for security)
gh secret list

# Expected output:
# AWS_ACCESS_KEY_ID       Updated YYYY-MM-DD
# AWS_SECRET_ACCESS_KEY   Updated YYYY-MM-DD
# EC2_SSH_PRIVATE_KEY     Updated YYYY-MM-DD
```

## 🔒 Security Best Practices

### 1. Use IAM User with Minimal Permissions

Create a dedicated IAM user for GitHub Actions with only the necessary permissions:

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "ec2:DescribeInstances",
                "ec2:StartInstances",
                "ec2:StopInstances",
                "ec2:RebootInstances",
                "ec2:CreateImage",
                "ec2:DescribeImages",
                "ec2:CreateTags",
                "ec2:AuthorizeSecurityGroupIngress",
                "ec2:DescribeSecurityGroups"
            ],
            "Resource": "*"
        },
        {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
                "s3:ListBucket"
            ],
            "Resource": [
                "arn:aws:s3:::dattavani",
                "arn:aws:s3:::dattavani/*"
            ]
        }
    ]
}
```

### 2. Rotate Keys Regularly

- Rotate AWS access keys every 90 days
- Update SSH keys if compromised
- Monitor AWS CloudTrail for unusual activity

### 3. Use Environment Protection Rules

Set up environment protection rules in GitHub:

1. Go to **Settings** → **Environments**
2. Create environments: `production`, `staging`
3. Add protection rules:
   - Required reviewers for production
   - Deployment branches (main for production, develop for staging)

## 🧪 Test the Setup

After setting up secrets, test the deployment:

1. **Push to develop branch** (triggers staging deployment):
   ```bash
   git checkout develop
   git push origin develop
   ```

2. **Push to main branch** (triggers production deployment):
   ```bash
   git checkout main
   git push origin main
   ```

3. **Manual deployment** (using workflow_dispatch):
   - Go to **Actions** tab in GitHub
   - Select "Deploy Dattavani ASR to AWS" workflow
   - Click "Run workflow"
   - Choose environment and options

## 🚨 Troubleshooting

### Common Issues

1. **AWS Credentials Invalid**
   ```
   Error: The security token included in the request is invalid
   ```
   - Check AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY
   - Verify IAM user has necessary permissions

2. **SSH Connection Failed**
   ```
   Error: Permission denied (publickey)
   ```
   - Check EC2_SSH_PRIVATE_KEY format
   - Ensure the key corresponds to the EC2 instance key pair

3. **Instance Not Found**
   ```
   Error: InvalidInstanceID.NotFound
   ```
   - Verify the instance ID in workflow files
   - Check AWS region settings

### Debug Steps

1. **Check workflow logs**:
   - Go to **Actions** tab
   - Click on failed workflow run
   - Expand failed job steps

2. **Test AWS credentials locally**:
   ```bash
   export AWS_ACCESS_KEY_ID="your-key-id"
   export AWS_SECRET_ACCESS_KEY="your-secret-key"
   aws sts get-caller-identity
   ```

3. **Test SSH connection**:
   ```bash
   ssh -i ~/.ssh/your-key.pem ubuntu@your-instance-ip
   ```

## 📋 Secrets Checklist

- [ ] `AWS_ACCESS_KEY_ID` - Set and tested
- [ ] `AWS_SECRET_ACCESS_KEY` - Set and tested  
- [ ] `EC2_SSH_PRIVATE_KEY` - Set and tested
- [ ] IAM user has minimal required permissions
- [ ] Environment protection rules configured
- [ ] Test deployment successful

## 🔄 Updating Secrets

To update existing secrets:

```bash
# Update AWS credentials
gh secret set AWS_ACCESS_KEY_ID --body "new-access-key-id"
gh secret set AWS_SECRET_ACCESS_KEY --body "new-secret-access-key"

# Update SSH key
gh secret set EC2_SSH_PRIVATE_KEY < ~/.ssh/new-key.pem
```

## 📞 Support

If you encounter issues:

1. Check this guide first
2. Review GitHub Actions logs
3. Verify AWS permissions and connectivity
4. Contact the development team

---

**🔐 Security Note**: Never commit secrets to your repository. Always use GitHub Secrets for sensitive information.
