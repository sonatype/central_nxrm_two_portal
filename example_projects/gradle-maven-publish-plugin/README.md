# Gradle's built-in `maven-publish` plugin

- [Public
  Documentation](https://docs.gradle.org/current/userguide/publishing_maven.html)
- [Source
  Code](https://github.com/gradle/gradle/blob/bc54b5e82d94a66920fc0b68737f134f73e8d5f5/platforms/software/maven/src/main/java/org/gradle/api/publish/maven/tasks/PublishToMavenRepository.java#L56)

## Relevant Configurations

- Gradle version: 8.5

## Lessons Learned

Gradle's publish tasks are dynamically created and therefore cannot be named as
a dependency. Instead, you have to find tasks with the type
`PublishToMavenRepository` and modify them that way
([Documentation](https://docs.gradle.org/current/userguide/publishing_customization.html#sec:configuring_publishing_tasks)).

``` gradle
tasks.withType(PublishToMavenRepository).all {
    doLast {
        println('Published');
    }
}
```

This approach was abandoned due to the lack of a built-in Groovy HTTP library,
but might still be the foundation for a build plugin.

## Plugin Requests

### `PUT /service/local/staging/deploy/maven2/<file_path>`

This uploads files to the default deployment location and lets NXRM2 handle the
details of isolating them into repositories based on the implicit profile
selection strategy
([Documentation](https://help.sonatype.com/en/configuring-the-staging-suite.html)).

### `GET /service/local/staging/deploy/maven2/<file_path>`

This appears to be a check to make sure that the upload succeeded, but it does
not fail on `404` responses.

## Problems

### How to close the repository

Since the default plugin does not attempt to finish a repository profile, any
users who use the proxy in this configuration will encounter the issue of the
files never actually ending up in the Portal.

The frequency of this behavior is unknown relative to other deployment methods
that automatically finish and promote the staging release.

#### Explicit close

The simple answer is to create a new, non-NXRM2 endpoint that closes the
repository.

##### Pros

- Simple to implement
- Enables ease of customization (e.g. auto v. manual release as query
  parameters)
- Relatively easy for users
   - Potentially provide a Gradle plugin
   - Easy to script with `curl`

##### Cons

- Doesn't "just work" for existing publishers
- High potential for confusion

#### Implicit close

Close the repository after a period of time without any new files added.

##### Pros

- "Just works" for existing publishers

##### Cons

- Increased publishing latency
- No way to have CI potentially check the status of a release to mark itself as
  failed
- Potential risk with a low timeout to miss including files in a release

##### Synthesis

Add the endpoint for users to be explicit, cleanup the repository after an hour
of inactivity by publishing a manually managed deployment to the Portal.
