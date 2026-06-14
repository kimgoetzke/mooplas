output "elastic_ip" {
  description = "Elastic IP associated with the signalling EC2 instance."
  value       = aws_eip.signalling.public_ip
}

output "ec2_public_dns" {
  description = "Public DNS name for the Elastic IP origin."
  value       = aws_eip.signalling.public_dns
}

output "cloudfront_domain_name" {
  description = "CloudFront distribution domain name. Wait until the distribution is deployed before using it."
  value       = aws_cloudfront_distribution.signalling.domain_name
}

output "signalling_server_url" {
  description = "WebSocket URL for SIGNALLING_SERVER_URL in production browser builds."
  value       = "wss://${aws_cloudfront_distribution.signalling.domain_name}"
}

output "ssh_command" {
  description = "Example SSH command. Replace the key path if needed."
  value       = "ssh -i <path-to-private-key> ec2-user@${aws_eip.signalling.public_ip} -v -o IdentitiesOnly=yes"
}
