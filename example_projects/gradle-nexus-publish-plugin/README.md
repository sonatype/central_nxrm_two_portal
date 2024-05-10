# Gradle-nexus's `publish-plugin` plugin

- [Source
  Code](https://github.com/gradle-nexus/publish-plugin/)

## Relevant Configurations

- Gradle version: 8.5
- Plugin version: 2.0.0

## Plugin Requests

### Interesting Headers

- `Authorization`: `Basic`
- `Accept`: `application/json`
- `User-agent`: `gradle-nexus-publish-plugin/2.0.0`

### `GET /service/local/staging/profiles`

- [NXRM2
  Documentation](https://s01.oss.sonatype.org/nexus-staging-plugin/default/docs/path__staging_profiles.html)
- [Calling
  code](https://github.com/gradle-nexus/publish-plugin/blob/c9b561160883d9e230e087dcb4fa45f6aeb44874/src/main/kotlin/io/github/gradlenexus/publishplugin/internal/NexusClient.kt#L74)

This appears to be an approach to getting all of the profiles that a user has
access to, and then filtering based on the profile name being a prefix of the
package group.

``` json
{
  "data": [
    {
      "resourceURI": "https://s01.oss.sonatype.org/service/local/staging/profiles/42704302172924",
      "id": "42704302172924",
      "name": "com.sonatype.central.testing.internal",
      "repositoryTemplateId": "default_hosted_release",
      "repositoryType": "maven2",
      "repositoryTargetId": "4270416802e184",
      "inProgress": false,
      "order": 18148,
      "deployURI": "https://s01.oss.sonatype.org/service/local/staging/deploy/maven2",
      "targetGroups": [
        "staging"
      ],
      "finishNotifyRoles": [
        "com.sonatype.central.testing.internal-deployer"
      ],
      "promotionNotifyRoles": [],
      "dropNotifyRoles": [],
      "closeRuleSets": [
        "99c4c121590a"
      ],
      "promoteRuleSets": [],
      "promotionTargetRepository": "releases",
      "mode": "BOTH",
      "finishNotifyCreator": true,
      "promotionNotifyCreator": true,
      "dropNotifyCreator": true,
      "autoStagingDisabled": false,
      "repositoriesSearchable": false,
      "properties": {
        "@class": "linked-hash-map"
      }
    }
  ]
}
```
