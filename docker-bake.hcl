group "default" {
  targets = [ "smppgc" ]
}

target "smppgc" {
  platforms = [ "linux/arm/v7" ]
  args = {
    RELEASE = "true"
  }

  target = "prod"
  tags = ["smppserver_smppgc:beta"]
}
