## AWS EC2 IMDSv2 Get Tool

This simple tool is useful for fetching data from imds on ec2 instances.
It will automatically check if imdsv2 is enabled and if so will utilize it instead
of imdsv1.

To use the tool simply call the cli and as its argument the path you want to extrac tinformation for.

Usage examples:
```
# Get the user data script
aws-ec2-imdsv2-get latest/user-data
```

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This project is licensed under the Apache-2.0 License.

