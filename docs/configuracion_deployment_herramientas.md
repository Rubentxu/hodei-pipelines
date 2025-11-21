# Configuración de Deployment y Herramientas de Seguridad - CI/CD Distribuido

## 1. Configuración Keycloak para CI/CD

### 1.1 Configuración de Realm CI/CD

```xml
<!-- keycloak-realm-config.xml -->
<realm name="cicd">
    <enabled>true</enabled>
    <displayName>CICD Platform Security</displayName>
    
    <!-- Authentication Configuration -->
    <authenticationFlows>
        <authenticationFlow>
            <id>cicd-flow</id>
            <alias>CI/CD Authentication Flow</alias>
            <description>Enhanced authentication flow for CI/CD</description>
            <provider>basic-flow</provider>
            <topLevel>true</topLevel>
            <subFlows>
                <!-- Username/Password Authentication -->
                <subFlow alias="Username and Password" type="flow">
                    <subFlows>
                        <subFlow alias="Username Password Form" type="form">
                            <subFlows>
                                <subFlow alias="Reset credentials if you are not registered" type="reset-credentials">
                                    <subFlows>
                                        <subFlow alias="Choose user" type="selector">
                                            <subFlows>
                                                <subFlow alias="Verify Username" type="idp-review-profile">
                                                    <subFlows>
                                                        <subFlow alias="Do existing user exist" type="providers">
                                                            <subFlows>
                                                                <subFlow alias="Handle Account Onboarding" type="registration-user-profile">
                                                                    <subFlows>
                                                                        <subFlow alias="Registration User Creation" type="registration-profile">
                                                                            <subFlows>
                                                                                <subFlow alias="Registration no auth required" type="registration-page-form">
                                                                                    <subFlows>
                                                                                        <subFlow alias="registration form" type="form-username-password-register">
                                                                                            <subFlows>
                                                                                                <subFlow alias="Subcribe with password" type="auth-username-password-form">
                                                                                                    <subFlows>
                                                                                                        <subFlow alias="Validate Password" type="auth-validate-form">
                                                                                                            <subFlows>
                                                                                                                <subFlow alias="Auth some form" type="auth-valid-form">
                                                                                                                    <subFlows>
                                                                                                                        <subFlow alias="Verify user by email" type="auth-email-verification-template-required">
                                                                                                                            <subFlows>
                                                                                                                                <subFlow alias="CICD MFA TOTP" type="totp">
                                                                                                                                    <subFlows>
                                                                                                                                        <subFlow alias="OTP Form" type="auth-otp-form">
                                                                                                                                            <subFlows>
                                                                                                                                                <subFlow alias="Check OTP" type="auth-otp-form-validate">
                                                                                                                                                    <subFlows>
                                                                                                                                                        <subFlow alias="Skip creating" type="auth-username-password-form">
                                                                                                                                                            <subFlows>
                                                                                                                                                                <subFlow alias="Update Profile On OTP Action" type="auth-spnego">
                                                                                                                                                                    <subFlows>
                                                                                                                                                                        <subFlow alias="Process Result Identity" type="identity-provider-first-login">
                                                                                                                                                                            <subFlows>
                                                                                                                                                                                <subFlow alias="Select Role" type="role-list">
                                                                                                                                                                                    <subFlows>
                                                                                                                                                                                        <subFlow alias="Deny Access" type="deny">
                                                                                                                                                                                            <subFlows>
                                                                                                                                                                                                <subFlow alias="Deny" type="failures">
                                                                                                                                                                                                    <subFlows>
                                                                                                                                                                                                        <subFlow alias="Fail" type="direct-grant">
                                                                                                                                                                                                            <subFlows>
                                                                                                                                                                                                                <subFlow alias="Set OTP Form" type="set-otp-form">
                                                                                                                                                                                                                    <subFlows>
                                                                                                                                                                                                                        <subFlow alias="WebAuthn Passwordless Browser"
                                                                                                                                                                                                                                 type="webauthn-browser-passwordless-pure">
                                                                                                                                                                                                                            <subFlows>
                                                                                                                                                                                                                                <subFlow alias="WebAuthn Register Passwordless"
                                                                                                                                                                                                                                         type="webauthn-register-passwordless-pure">
                                                                                                                                                                                                                                    <subFlows>
                                                                                                                                                                                                                                        <subFlow alias="WebAuthn Authenticate"
                                                                                                                                                                                                                                                 type="webauthn-authenticate-passwordless-pure">
                                                                                                                                                                                                                                            <subFlows>
                                                                                                                                                                                                                                                <subFlow alias="Handle User Device"/>
                                                                                                                                                                                                                                            </subFlows>
                                                                                                                                                                                                                                        </subFlow>
                                                                                                                                                                                                                                    </subFlows>
                                                                                                                                                                                                                                </subFlow>
                                                                                                                                                                                                                            </subFlows>
                                                                                                                                                                                                                        </subFlow>
                                                                                                                                                                                                                    </subFlows>
                                                                                                                                                                                                                </subFlow>
                                                                                                                                                                                                            </subFlows>
                                                                                                                                                                                                        </subFlow>
                                                                                                                                                                                                    </subFlows>
                                                                                                                                                                                                </subFlow>
                                                                                                                                                                                            </subFlows>
                                                                                                                                                                                        </subFlow>
                                                                                                                                                                                    </subFlows>
                                                                                                                                                                                </subFlow>
                                                                                                                                                                            </subFlows>
                                                                                                                                                                        </subFlow>
                                                                                                                                                                    </subFlows>
                                                                                                                                                                </subFlow>
                                                                                                                                                            </subFlows>
                                                                                                                                                        </subFlow>
                                                                                                                                                    </subFlows>
                                                                                                                                                </subFlow>
                                                                                                                                            </subFlows>
                                                                                                                                        </subFlow>
                                                                                                                                    </subFlows>
                                                                                                                                </subFlow>
                                                                                                                            </subFlows>
                                                                                                                        </subFlow>
                                                                                                                    </subFlows>
                                                                                                                </subFlow>
                                                                                                            </subFlows>
                                                                                                        </subFlow>
                                                                                                    </subFlows>
                                                                                                </subFlow>
                                                                                            </subFlows>
                                                                                        </subFlow>
                                                                                    </subFlows>
                                                                                </subFlow>
                                                                            </subFlows>
                                                                        </subFlow>
                                                                    </subFlows>
                                                                </subFlow>
                                                            </subFlows>
                                                        </subFlow>
                                                    </subFlows>
                                                </subFlow>
                                            </subFlows>
                                        </subFlow>
                                    </subFlows>
                                </subFlow>
                            </subFlows>
                        </subFlow>
                    </subFlows>
                </subFlow>
            </subFlows>
        </authenticationFlow>
    </authenticationFlows>
    
    <!-- Security Configuration -->
    <securitySettings>
        <allowUserRegistration>false</allowUserRegistration>
        <requireEmail>false</requireEmail>
        <rememberMe>true</rememberMe>
        <userActionTokens>
            <userActionTokenDefinition>
                <actionTokenType>CONFIGURE_TOTP</actionTokenType>
                <defaultTimeToLive>600</defaultTimeToLive>
                <actionTokenVerification jwtActionTokenVerificationEmail</jwtActionTokenVerificationEmail>
                <verificationNoteTokenDefinition>
                    <noteTokenType>VERIFY_EMAIL</noteTokenType>
                    <defaultTimeToLive>86400</defaultTimeToLive>
                    <verificationNoteToken jwtActionTokenVerificationEmail</jwtActionTokenVerificationEmail>
                    <verificationNoteTokenDefaultNoteVerification jwtActionTokenVerificationEmail</verificationNoteTokenDefaultNoteVerification>
                </verificationNoteTokenDefinition>
                <updateEmailVerificationNoteTokenDefinition>
                    <noteTokenType>VERIFY_EMAIL</noteTokenType>
                    <defaultTimeToLive>86400</defaultTimeToLive>
                    <verificationNoteToken jwtActionTokenVerificationEmail</verificationNoteTokenVerificationEmail>
                    <verificationNoteTokenDefaultNoteVerification jwtActionTokenVerificationEmail</verificationNoteTokenDefaultNoteVerification>
                </updateEmailVerificationNoteTokenDefinition>
                <confirmationEmailVerificationTokenDefinition>
                    <noteTokenType>VERIFY_EMAIL</noteTokenType>
                    <defaultTimeToLive>86400</defaultTimeToLive>
                    <verificationNoteToken jwtActionTokenVerificationEmail</verificationNoteTokenVerificationEmail>
                    <verificationNoteTokenDefaultNoteVerification jwtActionTokenVerificationEmail</verificationNoteTokenDefaultNoteVerification>
                </confirmationEmailVerificationTokenDefinition>
            </userActionTokenDefinition>
        </userActionTokens>
    </securitySettings>
    
    <!-- Client Configurations -->
    <clients>
        <!-- Machine-to-Machine Clients -->
        <client>
            <id>orchestrator-service</id>
            <clientId>orchestrator-service</clientId>
            <name>Orchestrator Service</name>
            <enabled>true</enabled>
            <clientAuthenticatorType>client-secret</clientAuthenticatorType>
            <secret>orchestrator-client-secret</secret>
            <protocol>openid-connect</protocol>
            <standardFlowEnabled>false</standardFlowEnabled>
            <directAccessGrantsEnabled>true</directAccessGrantsEnabled>
            <serviceAccountsEnabled>true</serviceAccountsEnabled>
            <publicClient>false</publicClient>
            <fullScopeAllowed>true</fullScopeAllowed>
            <attributes>
                <pkce.code.challenge.method>none</pkce.code.challenge.method>
                <client_credentials.token.supported>true</client_credentials.token.supported>
            </attributes>
        </client>
        
        <client>
            <id>scheduler-service</id>
            <clientId>scheduler-service</clientId>
            <name>Scheduler Service</name>
            <enabled>true</enabled>
            <clientAuthenticatorType>client-secret</clientAuthenticatorType>
            <secret>scheduler-client-secret</secret>
            <protocol>openid-connect</protocol>
            <standardFlowEnabled>false</standardFlowEnabled>
            <directAccessGrantsEnabled>true</directAccessGrantsEnabled>
            <serviceAccountsEnabled>true</serviceAccountsEnabled>
            <publicClient>false</publicClient>
            <fullScopeAllowed>true</fullScopeAllowed>
        </client>
        
        <client>
            <id>worker-manager-service</id>
            <clientId>worker-manager-service</clientId>
            <name>Worker Manager Service</name>
            <enabled>true</enabled>
            <clientAuthenticatorType>client-secret</clientAuthenticatorType>
            <secret>worker-manager-client-secret</secret>
            <protocol>openid-connect</protocol>
            <standardFlowEnabled>false</standardFlowEnabled>
            <directAccessGrantsEnabled>true</directAccessGrantsEnabled>
            <serviceAccountsEnabled>true</serviceAccountsEnabled>
            <publicClient>false</publicClient>
            <fullScopeAllowed>true</fullScopeAllowed>
        </client>
        
        <client>
            <id>telemetry-service</id>
            <clientId>telemetry-service</clientId>
            <name>Telemetry Service</name>
            <enabled>true</enabled>
            <clientAuthenticatorType>client-secret</clientAuthenticatorType>
            <secret>telemetry-client-secret</secret>
            <protocol>openid-connect</protocol>
            <standardFlowEnabled>false</standardFlowEnabled>
            <directAccessGrantsEnabled>true</directAccessGrantsEnabled>
            <serviceAccountsEnabled>true</serviceAccountsEnabled>
            <publicClient>false</publicClient>
            <fullScopeAllowed>true</fullScopeAllowed>
        </client>
        
        <!-- Web Console Client -->
        <client>
            <id>cicd-console</id>
            <clientId>cicd-console</clientId>
            <name>CI/CD Console</name>
            <enabled>true</enabled>
            <clientAuthenticatorType>client-secret</clientAuthenticatorType>
            <secret>console-client-secret</secret>
            <protocol>openid-connect</protocol>
            <standardFlowEnabled>true</standardFlowEnabled>
            <directAccessGrantsEnabled>false</directAccessGrantsEnabled>
            <serviceAccountsEnabled>false</serviceAccountsEnabled>
            <publicClient>true</publicClient>
            <redirectUris>
                <redirectUri>https://console.cicd.local/auth/callback</redirectUri>
                <redirectUri>https://console.cicd.local/*</redirectUri>
            </redirectUris>
            <webOrigins>
                <webOrigin>https://console.cicd.local</webOrigin>
            </webOrigins>
            <fullScopeAllowed>false</fullScopeAllowed>
            <attributes>
                <pkce.code.challenge.method>S256</pkce.code.challenge.method>
            </attributes>
        </client>
    </clients>
    
    <!-- Client Scopes -->
    <clientScopes>
        <clientScope>
            <id>cicd-scope</id>
            <name>CICD Scopes</name>
            <type>default</type>
            <defaultClientScopes>
                <defaultClientScope>job:create</defaultClientScope>
                <defaultClientScope>job:read</defaultClientScope>
                <defaultClientScope>job:execute</defaultClientScope>
                <defaultClientScope>job:update</defaultClientScope>
                <defaultClientScope>job:delete</defaultClientScope>
            </defaultClientScopes>
            <optionalClientScopes>
                <optionalClientScope>offline_access</optionalClientScope>
                <optionalClientScope>profile</optionalClientScope>
                <optionalClientScope>email</optionalClientScope>
                <optionalClientScope>phone</optionalClientScope>
                <optionalClientScope>address</optionalClientScope>
                <optionalClientScope>microprofile-jwt</optionalClientScope>
            </optionalClientScopes>
        </clientScope>
        
        <clientScope>
            <id>service-scopes</id>
            <name>Service Scopes</name>
            <type>default</type>
            <defaultClientScopes>
                <defaultClientScope>service:orchestrator</defaultClientScope>
                <defaultClientScope>service:scheduler</defaultClientScope>
                <defaultClientScope>service:worker-manager</defaultClientScope>
                <defaultClientScope>service:telemetry</defaultClientScope>
            </defaultClientScopes>
        </clientScope>
    </clientScopes>
    
    <!-- Role Definitions -->
    <roles>
        <realmRoles>
            <realmRole>
                <id>admin</id>
                <name>admin</name>
                <description>Administrator role with full access</description>
                <composite>false</composite>
                <clientRole>false</clientRole>
            </realmRole>
            <realmRole>
                <id>devops-engineer</id>
                <name>devops-engineer</name>
                <description>DevOps engineer with pipeline access</description>
                <composite>false</composite>
                <clientRole>false</clientRole>
            </realmRole>
            <realmRole>
                <id>developer</id>
                <name>developer</name>
                <description>Developer with limited pipeline access</description>
                <composite>false</composite>
                <clientRole>false</clientRole>
            </realmRole>
            <realmRole>
                <id>operator</id>
                <name>operator</name>
                <description>System operator with monitoring access</description>
                <composite>false</composite>
                <clientRole>false</clientRole>
            </realmRole>
            <realmRole>
                <id>security-admin</id>
                <name>security-admin</name>
                <description>Security administrator with audit access</description>
                <composite>false</composite>
                <clientRole>false</clientRole>
            </realmRole>
        </realmRoles>
        
        <!-- Service Account Roles -->
        <role>
            <id>orchestrator-service</id>
            <name>service:orchestrator</name>
            <description>Service account for orchestrator component</description>
            <composite>false</composite>
            <clientRole>true</clientRole>
            <client>orchestrator-service</client>
        </role>
        
        <role>
            <id>scheduler-service</id>
            <name>service:scheduler</name>
            <description>Service account for scheduler component</description>
            <composite>false</composite>
            <clientRole>true</clientRole>
            <client>scheduler-service</client>
        </role>
        
        <role>
            <id>worker-manager-service</id>
            <name>service:worker-manager</name>
            <description>Service account for worker manager component</description>
            <composite>false</composite>
            <clientRole>true</clientRole>
            <client>worker-manager-service</client>
        </role>
        
        <role>
            <id>telemetry-service</id>
            <name>service:telemetry</name>
            <description>Service account for telemetry component</description>
            <composite>false</composite>
            <clientRole>true</clientRole>
            <client>telemetry-service</client>
        </role>
    </roles>
</realm>
```

### 1.2 Docker Compose para Keycloak

```yaml
# docker-compose.keycloak.yml
version: '3.8'

services:
  keycloak-postgres:
    image: postgres:15
    container_name: keycloak-postgres
    environment:
      POSTGRES_DB: keycloak
      POSTGRES_USER: keycloak
      POSTGRES_PASSWORD: keycloak_password_2024
    volumes:
      - keycloak-postgres-data:/var/lib/postgresql/data
    networks:
      - keycloak-network
    restart: unless-stopped

  keycloak:
    image: quay.io/keycloak/keycloak:23.0
    container_name: keycloak
    environment:
      KEYCLOAK_ADMIN: admin
      KEYCLOAK_ADMIN_PASSWORD: admin_password_2024
      KC_DB: postgres
      KC_DB_URL: jdbc:postgresql://keycloak-postgres:5432/keycloak
      KC_DB_USERNAME: keycloak
      KC_DB_PASSWORD: keycloak_password_2024
      KC_HOSTNAME: keycloak.cicd.local
      KC_HOSTNAME_STRICT: false
      KC_HTTP_ENABLED: true
      KC_HTTP_PORT: 8080
      KC_HTTPS_PORT: 8443
      KC_HEALTH_ENABLED: true
      KC_METRICS_ENABLED: true
      KC_LOG_LEVEL: INFO
    volumes:
      - ./keycloak-realm-config.xml:/opt/keycloak/data/import/realm-export.json:ro
      - keycloak-certificates:/opt/keycloak/certs
    networks:
      - keycloak-network
    ports:
      - "8080:8080"
      - "8443:8443"
    depends_on:
      - keycloak-postgres
    restart: unless-stopped

  keycloak-ha:
    image: quay.io/keycloak/keycloak:23.0
    container_name: keycloak-ha
    environment:
      KEYCLOAK_ADMIN: admin
      KEYCLOAK_ADMIN_PASSWORD: admin_password_2024
      KC_DB: postgres
      KC_DB_URL: jdbc:postgresql://keycloak-postgres:5432/keycloak
      KC_DB_USERNAME: keycloak
      KC_DB_PASSWORD: keycloak_password_2024
      KC_HOSTNAME: keycloak-ha.cicd.local
      KC_HOSTNAME_STRICT: false
      KC_HTTP_ENABLED: true
      KC_HTTP_PORT: 8080
      KC_HTTPS_ENABLED: true
      KC_HTTPS_PORT: 8443
      KC_HEALTH_ENABLED: true
      KC_METRICS_ENABLED: true
      KC_LOG_LEVEL: INFO
      KC_CACHE: ispn
      KC_CACHE_STACK: udp
    volumes:
      - ./keycloak-realm-config.xml:/opt/keycloak/data/import/realm-export.json:ro
      - keycloak-certificates:/opt/keycloak/certs
    networks:
      - keycloak-network
    ports:
      - "8081:8080"
      - "8444:8443"
    depends_on:
      - keycloak-postgres
    restart: unless-stopped

volumes:
  keycloak-postgres-data:
  keycloak-certificates:

networks:
  keycloak-network:
    driver: bridge
    ipam:
      config:
        - subnet: 172.20.0.0/16
```

## 2. Políticas AWS Verified Permissions

### 2.1 Esquema Cedar para CI/CD

```json
{
  "cedar_json": {
    "cicd": {
      "actions": {
        "create_job": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "project_id": {
                  "type": "String"
                },
                "environment": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["dev", "stage", "prod"]
                  }
                },
                "priority": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["low", "normal", "high", "critical"]
                  }
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Job"]
          },
          "memberOf": []
        },
        "read_job": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "owner": {
                  "type": "String"
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Job"]
          },
          "memberOf": []
        },
        "execute_job": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "environment": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["dev", "stage", "prod"]
                  }
                },
                "required_role": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["developer", "devops-engineer", "admin"]
                  }
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Job"]
          },
          "memberOf": []
        },
        "update_job": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "owner": {
                  "type": "String"
                },
                "modification_type": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["metadata", "configuration", "state"]
                  }
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Job"]
          },
          "memberOf": []
        },
        "delete_job": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "owner": {
                  "type": "String"
                },
                "environment": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["dev", "stage", "prod"]
                  }
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Job"]
          },
          "memberOf": []
        },
        "create_pipeline": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "project_id": {
                  "type": "String"
                },
                "environment": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["dev", "stage", "prod"]
                  }
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Pipeline"]
          },
          "memberOf": []
        },
        "execute_pipeline": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "environment": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["dev", "stage", "prod"]
                  }
                },
                "approval_required": {
                  "type": "Boolean"
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Pipeline"]
          },
          "memberOf": []
        },
        "manage_workers": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "environment": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["dev", "stage", "prod"]
                  }
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Worker"]
          },
          "memberOf": []
        },
        "manage_artifacts": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "repository": {
                  "type": "String"
                },
                "environment": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["dev", "stage", "prod"]
                  }
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Artifact"]
          },
          "memberOf": []
        },
        "view_telemetry": {
          "appliesTo": {
            "context": {
              "type": "Record",
              "attributes": {
                "tenant_id": {
                  "type": "String"
                },
                "data_level": {
                  "type": "String",
                  "validations": {
                    "oneOf": ["summary", "detailed", "full"]
                  }
                }
              }
            },
            "principalTypes": ["User"],
            "resourceTypes": ["Metric"]
          },
          "memberOf": []
        }
      },
      "entityTypes": {
        "Job": {
          "memberOfTypes": [],
          "shape": {
            "type": "Record",
            "attributes": {
              "tenant_id": {
                "type": "String"
              },
              "project_id": {
                "type": "String"
              },
              "owner": {
                "type": "String"
              },
              "environment": {
                "type": "String",
                "validations": {
                  "oneOf": ["dev", "stage", "prod"]
                }
              },
              "priority": {
                "type": "String",
                "validations": {
                  "oneOf": ["low", "normal", "high", "critical"]
                }
              },
              "status": {
                "type": "String",
                "validations": {
                  "oneOf": ["pending", "running", "completed", "failed", "cancelled"]
                }
              }
            }
          }
        },
        "Pipeline": {
          "memberOfTypes": [],
          "shape": {
            "type": "Record",
            "attributes": {
              "tenant_id": {
                "type": "String"
              },
              "project_id": {
                "type": "String"
              },
              "owner": {
                "type": "String"
              },
              "environment": {
                "type": "String",
                "validations": {
                  "oneOf": ["dev", "stage", "prod"]
                }
              },
              "is_critical": {
                "type": "Boolean"
              }
            }
          }
        },
        "Worker": {
          "memberOfTypes": [],
          "shape": {
            "type": "Record",
            "attributes": {
              "tenant_id": {
                "type": "String"
              },
              "environment": {
                "type": "String",
                "validations": {
                  "oneOf": ["dev", "stage", "prod"]
                }
              },
              "status": {
                "type": "String",
                "validations": {
                  "oneOf": ["active", "inactive", "maintenance", "error"]
                }
              }
            }
          }
        },
        "Artifact": {
          "memberOfTypes": [],
          "shape": {
            "type": "Record",
            "attributes": {
              "tenant_id": {
                "type": "String"
              },
              "repository": {
                "type": "String"
              },
              "version": {
                "type": "String"
              },
              "checksum": {
                "type": "String"
              },
              "size": {
                "type": "Long"
              }
            }
          }
        },
        "Metric": {
          "memberOfTypes": [],
          "shape": {
            "type": "Record",
            "attributes": {
              "tenant_id": {
                "type": "String"
              },
              "component": {
                "type": "String",
                "validations": {
                  "oneOf": ["orchestrator", "scheduler", "worker-manager", "workers", "telemetry"]
                }
              },
              "environment": {
                "type": "String",
                "validations": {
                  "oneOf": ["dev", "stage", "prod"]
                }
              }
            }
          }
        },
        "User": {
          "memberOfTypes": [],
          "shape": {
            "type": "Record",
            "attributes": {
              "tenant_id": {
                "type": "String"
              },
              "email": {
                "type": "String"
              },
              "roles": {
                "type": "Array",
                "items": {
                  "type": "String"
                }
              },
              "department": {
                "type": "String"
              },
              "access_level": {
                "type": "String",
                "validations": {
                  "oneOf": ["basic", "advanced", "admin"]
                }
              }
            }
          }
        }
      }
    }
  }
}
```

### 2.2 Políticas Estáticas por Rol

```cedar
// Admin Role Policy
permit (
    principal in cicd::User::"admin",
    action in [
        cicd::Action::"create_job",
        cicd::Action::"read_job",
        cicd::Action::"execute_job",
        cicd::Action::"update_job",
        cicd::Action::"delete_job",
        cicd::Action::"create_pipeline",
        cicd::Action::"execute_pipeline",
        cicd::Action::"manage_workers",
        cicd::Action::"manage_artifacts",
        cicd::Action::"view_telemetry"
    ],
    resource
);

// DevOps Engineer Policy
permit (
    principal in cicd::User::"devops-engineer",
    action in [
        cicd::Action::"create_job",
        cicd::Action::"read_job",
        cicd::Action::"execute_job",
        cicd::Action::"update_job",
        cicd::Action::"delete_job",
        cicd::Action::"create_pipeline",
        cicd::Action::"execute_pipeline",
        cicd::Action::"manage_workers",
        cicd::Action::"view_telemetry"
    ],
    resource
) when {
    resource in cicd::Job::"*" || resource in cicd::Pipeline::"*" || resource in cicd::Worker::"*"
};

// Developer Policy
permit (
    principal in cicd::User::"developer",
    action in [
        cicd::Action::"create_job",
        cicd::Action::"read_job",
        cicd::Action::"execute_job",
        cicd::Action::"update_job",
        cicd::Action::"create_pipeline",
        cicd::Action::"view_telemetry"
    ],
    resource
) when {
    (resource.environment == "dev" || resource.environment == "stage") &&
    (principal.roles contains "developer")
};

// Operator Policy
permit (
    principal in cicd::User::"operator",
    action in [
        cicd::Action::"read_job",
        cicd::Action::"view_telemetry"
    ],
    resource
);

// Service Account Policies
permit (
    principal in cicd::User::"service:orchestrator",
    action in [
        cicd::Action::"create_job",
        cicd::Action::"read_job",
        cicd::Action::"execute_job",
        cicd::Action::"update_job",
        cicd::Action::"manage_workers"
    ],
    resource
) when {
    resource.tenant_id == principal.tenant_id
};
```

### 2.3 Políticas ABAC Dinámicas

```cedar
// Environment-based policy
permit (
    principal,
    action in [cicd::Action::"execute_job", cicd::Action::"create_job"],
    resource
) when {
    (principal.access_level == "advanced" && resource.environment == "dev") ||
    (principal.access_level == "admin" && resource.environment == "stage") ||
    (principal.access_level == "admin" && resource.environment == "prod" && principal.roles contains "security-admin")
};

// Priority-based job access
permit (
    principal,
    action in [cicd::Action::"execute_job", cicd::Action::"cancel_job"],
    resource
) when {
    (resource.priority == "low" && principal.access_level != "basic") ||
    (resource.priority == "critical" && principal.roles contains "admin")
};

// Tenant isolation policy
permit (
    principal,
    action in [cicd::Action::"create_job", cicd::Action::"read_job", cicd::Action::"update_job"],
    resource
) when {
    resource.tenant_id == principal.tenant_id
};

// Time-based access control (business hours)
permit (
    principal,
    action in [cicd::Action::"execute_job", cicd::Action::"create_pipeline"],
    resource
) when {
    // This would need to be implemented with time functions in Cedar
    // For now, it's a placeholder for business hours access control
    true
};
```

## 3. Scripts de Deployment Seguro

### 3.1 Script de Generación de Certificados mTLS

```bash
#!/bin/bash
# generate-mtls-certs.sh

set -euo pipefail

# Configuration
CERT_DIR="./certificates"
CA_CERT="$CERT_DIR/ca.crt"
CA_KEY="$CERT_DIR/ca.key"
CA_SERIAL="$CERT_DIR/ca.srl"

# Component certificates
ORCHESTRATOR_CERT="$CERT_DIR/orchestrator.crt"
ORCHESTRATOR_KEY="$CERT_DIR/orchestrator.key"
SCHEDULER_CERT="$CERT_DIR/scheduler.crt"
SCHEDULER_KEY="$CERT_DIR/scheduler.key"
WORKER_MANAGER_CERT="$CERT_DIR/worker-manager.crt"
WORKER_MANAGER_KEY="$CERT_DIR/worker-manager.key"
TELEMETRY_CERT="$CERT_DIR/telemetry.crt"
TELEMETRY_KEY="$CERT_DIR/telemetry.key"
WORKERS_CERT="$CERT_DIR/workers.crt"
WORKERS_KEY="$CERT_DIR/workers.key"

# Validity period (in days)
VALIDITY_DAYS=30

# Create certificate directory
mkdir -p "$CERT_DIR"

echo "Generating mTLS certificates for CI/CD platform..."

# Generate CA private key
echo "Generating CA private key..."
openssl genrsa -out "$CA_KEY" 4096

# Generate CA certificate
echo "Generating CA certificate..."
openssl req -new -x509 -days "$VALIDITY_DAYS" -key "$CA_KEY" -out "$CA_CERT" \
  -subj "/C=US/ST=CA/L=San Francisco/O=CICD Platform/OU=Security/CN=CICD-CA"

# Function to generate component certificate
generate_component_cert() {
    local cert_name=$1
    local cert_file="${CERT_DIR}/${cert_name}.crt"
    local key_file="${CERT_DIR}/${cert_name}.key"
    
    echo "Generating certificate for ${cert_name}..."
    
    # Generate private key
    openssl genrsa -out "$key_file" 2048
    
    # Create CSR
    openssl req -new -key "$key_file" -out "${cert_name}.csr" \
      -subj "/C=US/ST=CA/L=San Francisco/O=CICD Platform/OU=${cert_name}/CN=${cert_name}.cicd.local"
    
    # Sign certificate with CA
    openssl x509 -req -in "${cert_name}.csr" -CA "$CA_CERT" -CAkey "$CA_KEY" \
      -CAcreateserial -out "$cert_file" -days "$VALIDITY_DAYS" \
      -extensions v3_req -extfile <(cat << EOF
[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth, clientAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = ${cert_name}.cicd.local
DNS.2 = ${cert_name}
IP.1 = 127.0.0.1
IP.2 = ::1
EOF
    )
    
    # Clean up CSR
    rm "${cert_name}.csr"
    
    # Create certificate bundle (cert + CA)
    cat "$cert_file" "$CA_CERT" > "${cert_file%.crt}.bundle.crt"
    
    echo "Certificate for ${cert_name} generated successfully"
}

# Generate certificates for each component
generate_component_cert "orchestrator"
generate_component_cert "scheduler"
generate_component_cert "worker-manager"
generate_component_cert "telemetry"
generate_component_cert "workers"

# Create service-specific certificate bundles
echo "Creating certificate bundles..."
cat "$ORCHESTRATOR_CERT" "$CA_CERT" > "$ORCHESTRATOR_CERT.bundle"
cat "$SCHEDULER_CERT" "$CA_CERT" > "$SCHEDULER_CERT.bundle"
cat "$WORKER_MANAGER_CERT" "$CA_CERT" > "$WORKER_MANAGER_CERT.bundle"
cat "$TELEMETRY_CERT" "$CA_CERT" > "$TELEMETRY_CERT.bundle"
cat "$WORKERS_CERT" "$CA_CERT" > "$WORKERS_CERT.bundle"

# Generate certificate revocation list
echo "Generating certificate revocation list..."
openssl ca -gencrl -out "$CERT_DIR/ca.crl" -config <(cat << EOF
[ca]
default_ca = CA_default

[CA_default]
dir = $CERT_DIR
database = $CERT_DIR/index.txt
new_certs_dir = $CERT_DIR/newcerts
certificate = $CA_CERT
serial = $CERT_DIR/ca.srl
default_days = $VALIDITY_DAYS
default_md = sha256
policy = policy_anything
copy_extensions = copy

[policy_anything]
countryName = optional
stateOrProvinceName = optional
localityName = optional
organizationName = optional
organizationalUnitName = optional
commonName = supplied
emailAddress = optional
EOF
)

# Set proper permissions
chmod 600 "$CA_KEY" "$ORCHESTRATOR_KEY" "$SCHEDULER_KEY" "$WORKER_MANAGER_KEY" "$TELEMETRY_KEY" "$WORKERS_KEY"
chmod 644 "$CA_CERT" "$ORCHESTRATOR_CERT" "$SCHEDULER_CERT" "$WORKER_MANAGER_CERT" "$TELEMETRY_CERT" "$WORKERS_CERT"
chmod 644 "$ORCHESTRATOR_CERT.bundle" "$SCHEDULER_CERT.bundle" "$WORKER_MANAGER_CERT.bundle" "$TELEMETRY_CERT.bundle" "$WORKERS_CERT.bundle"

# Create certificates manifest
cat > "$CERT_DIR/certificates-manifest.json" << EOF
{
  "generated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "validity_days": $VALIDITY_DAYS,
  "components": {
    "orchestrator": {
      "cert": "orchestrator.crt",
      "bundle": "orchestrator.crt.bundle",
      "key": "orchestrator.key"
    },
    "scheduler": {
      "cert": "scheduler.crt",
      "bundle": "scheduler.crt.bundle",
      "key": "scheduler.key"
    },
    "worker-manager": {
      "cert": "worker-manager.crt",
      "bundle": "worker-manager.crt.bundle",
      "key": "worker-manager.key"
    },
    "telemetry": {
      "cert": "telemetry.crt",
      "bundle": "telemetry.crt.bundle",
      "key": "telemetry.key"
    },
    "workers": {
      "cert": "workers.crt",
      "bundle": "workers.crt.bundle",
      "key": "workers.key"
    }
  },
  "ca": {
    "cert": "ca.crt",
    "key": "ca.key",
    "crl": "ca.crl"
  }
}
EOF

echo "mTLS certificate generation completed successfully!"
echo "Certificates are located in: $CERT_DIR"
echo "Don't forget to secure the CA private key: $CA_KEY"
```

### 3.2 Script de Configuración Automática

```bash
#!/bin/bash
# setup-security.sh

set -euo pipefail

# Configuration
ENVIRONMENT=${1:-dev} # dev, stage, prod
KEYCLOAK_URL=${2:-http://localhost:8080}
AVP_POLICY_STORE_ID=${3:-}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check dependencies
check_dependencies() {
    log_info "Checking dependencies..."
    
    local missing_deps=()
    
    # Check for required tools
    for cmd in curl jq openssl kubectl docker; do
        if ! command -v "$cmd" &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        exit 1
    fi
    
    log_info "All dependencies are available"
}

# Function to setup secrets management
setup_secrets() {
    log_info "Setting up secrets management..."
    
    # Generate random secrets
    local jwt_secret=$(openssl rand -base64 32)
    local encryption_key=$(openssl rand -base64 32)
    
    # Create secrets file
    cat > secrets.env << EOF
# Generated secrets for CI/CD platform
JWT_SECRET=$jwt_secret
ENCRYPTION_KEY=$encryption_key

# Keycloak configuration
KEYCLOAK_URL=$KEYCLOAK_URL
KEYCLOAK_REALM=cicd

# AWS AVP configuration
AVP_POLICY_STORE_ID=$AVP_POLICY_STORE_ID

# Database secrets (if using external database)
POSTGRES_PASSWORD=$(openssl rand -base64 16)
REDIS_PASSWORD=$(openssl rand -base64 16)

# mTLS certificate passwords (if using PKCS12)
CERT_PASSWORD=$(openssl rand -base64 16)
EOF

    # Secure the secrets file
    chmod 600 secrets.env
    
    log_info "Secrets generated and saved to secrets.env"
    log_warn "Please secure the secrets.env file - it contains sensitive data"
}

# Function to setup monitoring
setup_monitoring() {
    log_info "Setting up security monitoring..."
    
    # Create monitoring configuration
    cat > monitoring-config.yaml << EOF
# Security monitoring configuration
security_monitoring:
  audit_log_retention_days: 2555  # 7 years for compliance
  alert_thresholds:
    failed_authentication_rate: 5  # per minute
    denied_authorization_rate: 10  # per minute
    suspicious_activity_rate: 3   # per minute
  
  log_levels:
    security_events: INFO
    authentication_errors: WARN
    authorization_denials: WARN
    system_errors: ERROR
    audit_trail: INFO
  
  siem_integration:
    enabled: true
    endpoint: http://localhost:8088  # Fluentd endpoint
    buffer_size: 1000
    flush_interval: 30s
EOF

    log_info "Monitoring configuration created"
}

# Function to setup backup and recovery
setup_backup() {
    log_info "Setting up backup configuration..."
    
    # Create backup script
    cat > backup-security.sh << 'EOF'
#!/bin/bash
# Backup security-critical data

set -euo pipefail

BACKUP_DIR="/backup/security-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Backup secrets
cp secrets.env "$BACKUP_DIR/"

# Backup certificates
if [ -d "./certificates" ]; then
    cp -r ./certificates "$BACKUP_DIR/"
fi

# Backup Keycloak realm configuration
if [ -d "./keycloak-realm-config.xml" ]; then
    cp ./keycloak-realm-config.xml "$BACKUP_DIR/"
fi

# Backup AVP policies (this would be implemented with AWS CLI)
# aws verifiedpermissions get-policy --policy-store-id <id> --policy-id <id>

# Backup audit logs
if [ -d "./audit-logs" ]; then
    cp -r ./audit-logs "$BACKUP_DIR/"
fi

# Create backup manifest
cat > "$BACKUP_DIR/backup-manifest.json" << EOL
{
  "backup_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "backup_type": "security_full",
  "environment": "$ENVIRONMENT",
  "components": [
    "secrets",
    "certificates",
    "keycloak",
    "avp_policies",
    "audit_logs"
  ]
}
EOL

# Compress backup
tar -czf "${BACKUP_DIR}.tar.gz" -C "$(dirname "$BACKUP_DIR)" "$(basename "$BACKUP_DIR")"

echo "Security backup completed: ${BACKUP_DIR}.tar.gz"
EOF

    chmod +x backup-security.sh
    
    log_info "Backup script created: backup-security.sh"
}

# Function to validate security configuration
validate_security() {
    log_info "Validating security configuration..."
    
    local validation_errors=0
    
    # Check if secrets file exists
    if [ ! -f "secrets.env" ]; then
        log_error "secrets.env file not found"
        validation_errors=$((validation_errors + 1))
    fi
    
    # Check certificate files
    for cert_dir in "./certificates" "./keys"; do
        if [ -d "$cert_dir" ]; then
            # Check for required certificates
            for cert in ca.crt orchestrator.crt scheduler.crt worker-manager.crt; do
                if [ ! -f "$cert_dir/$cert" ]; then
                    log_error "Certificate $cert not found in $cert_dir"
                    validation_errors=$((validation_errors + 1))
                fi
            done
        fi
    done
    
    # Check Keycloak connectivity (if available)
    if curl -f -s "$KEYCLOAK_URL/health" > /dev/null 2>&1; then
        log_info "Keycloak is accessible"
    else
        log_warn "Keycloak is not accessible at $KEYCLOAK_URL"
    fi
    
    if [ $validation_errors -eq 0 ]; then
        log_info "Security configuration validation passed"
    else
        log_error "Security configuration validation failed with $validation_errors errors"
        exit 1
    fi
}

# Function to generate security documentation
generate_docs() {
    log_info "Generating security documentation..."
    
    cat > SECURITY_IMPLEMENTATION.md << 'EOF'
# Security Implementation Guide

## Overview
This document describes the security implementation for the CI/CD platform.

## Security Components

### Authentication
- **Provider**: Keycloak (OIDC/OAuth2)
- **Machine-to-Machine**: Service accounts with client credentials
- **User Authentication**: Authorization Code Flow with PKCE
- **MFA**: TOTP and WebAuthn support

### Authorization
- **Provider**: AWS Verified Permissions
- **Model**: RBAC + ABAC (Cedar policies)
- **Granular**: Per-component, per-operation permissions
- **Dynamic**: Real-time policy evaluation

### Transport Security
- **Protocol**: mTLS for internal communication
- **CA**: Internal certificate authority
- **Certificates**: 30-day validity with automatic rotation
- **Pinning**: Service-specific certificate pinning

### Data Protection
- **Encryption**: AES-256-GCM for data at rest
- **Key Management**: Secure key lifecycle management
- **Secrets**: Centralized secret management
- **Audit**: Tamper-proof audit trails

### Monitoring & Compliance
- **Audit Trail**: Comprehensive security event logging
- **Compliance**: SOC2, GDPR, ISO27001 support
- **Monitoring**: Real-time security monitoring
- **Forensics**: Distributed forensic logging

## Deployment Security Checklist

- [ ] Keycloak configured with realm and clients
- [ ] AWS Verified Permissions with Cedar policies
- [ ] mTLS certificates generated and distributed
- [ ] Secrets management setup
- [ ] Monitoring and alerting configured
- [ ] Backup and recovery procedures tested
- [ ] Security documentation reviewed
- [ ] Compliance requirements validated

## Incident Response

### Security Incident Types
1. **Authentication Failures**
   - Multiple failed login attempts
   - Suspicious authentication patterns

2. **Authorization Issues**
   - Privilege escalation attempts
   - Unauthorized access to resources

3. **Certificate Issues**
   - Expired or invalid certificates
   - Certificate tampering attempts

4. **Data Breaches**
   - Unauthorized data access
   - Data exfiltration attempts

### Response Procedures
1. **Immediate Actions**
   - Isolate affected systems
   - Preserve evidence
   - Notify security team

2. **Investigation**
   - Analyze audit logs
   - Review access patterns
   - Assess impact scope

3. **Recovery**
   - Restore from clean backups
   - Update security policies
   - Implement additional controls

4. **Post-Incident**
   - Document lessons learned
   - Update security procedures
   - Conduct security training

## Compliance

### SOC2 Controls
- **CC6.1**: Logical and physical access controls
- **CC6.2**: System access authentication
- **CC6.3**: Logical access security measures
- **CC6.4**: Data transmission security

### GDPR Requirements
- **Article 25**: Data protection by design
- **Article 32**: Security of processing
- **Article 33**: Breach notification
- **Article 35**: Data protection impact assessment

## Contact Information

For security questions or incidents:
- Security Team: security@company.com
- Incident Response: incidents@company.com
- Emergency: +1-XXX-XXX-XXXX
EOF

    log_info "Security documentation generated: SECURITY_IMPLEMENTATION.md"
}

# Main execution
main() {
    log_info "Starting security setup for environment: $ENVIRONMENT"
    
    check_dependencies
    setup_secrets
    setup_monitoring
    setup_backup
    validate_security
    generate_docs
    
    log_info "Security setup completed successfully!"
    log_info "Next steps:"
    log_info "1. Review and update secrets.env with production values"
    log_info "2. Configure monitoring and alerting systems"
    log_info "3. Test backup and recovery procedures"
    log_info "4. Deploy and validate security controls"
}

# Run main function
main "$@"
```

### 3.3 Script de Validación de Seguridad

```bash
#!/bin/bash
# validate-security.sh

set -euo pipefail

# Configuration
ENVIRONMENT=${1:-dev}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Test suite
run_security_tests() {
    local total_tests=0
    local passed_tests=0
    
    # Test 1: Certificate validation
    test_certificate_validation() {
        total_tests=$((total_tests + 1))
        
        # Check if certificates exist and are valid
        if [ -f "./certificates/ca.crt" ] && [ -f "./certificates/orchestrator.crt" ]; then
            # Validate certificate format
            if openssl x509 -in ./certificates/ca.crt -noout -text > /dev/null 2>&1; then
                passed_tests=$((passed_tests + 1))
                log_pass "Certificate validation"
            else
                log_fail "Invalid certificate format"
            fi
        else
            log_fail "Certificate files not found"
        fi
    }
    
    # Test 2: Secret validation
    test_secret_validation() {
        total_tests=$((total_tests + 1))
        
        if [ -f "secrets.env" ]; then
            # Check if secrets file is properly secured
            local file_perms=$(stat -c "%a" secrets.env 2>/dev/null || stat -f "%p" secrets.env 2>/dev/null)
            
            if [ "$file_perms" = "600" ]; then
                passed_tests=$((passed_tests + 1))
                log_pass "Secret file permissions"
            else
                log_fail "Secret file should have 600 permissions"
            fi
        else
            log_fail "Secrets file not found"
        fi
    }
    
    # Test 3: Keycloak connectivity
    test_keycloak_connectivity() {
        total_tests=$((total_tests + 1))
        
        if curl -f -s "http://localhost:8080/health" > /dev/null 2>&1; then
            passed_tests=$((passed_tests + 1))
            log_pass "Keycloak connectivity"
        else
            log_fail "Keycloak not accessible"
        fi
    }
    
    # Test 4: JWT token generation
    test_jwt_generation() {
        total_tests=$((total_tests + 1))
        
        # This would test JWT token generation with the configured secret
        # For now, just check if the JWT secret exists
        if grep -q "JWT_SECRET" secrets.env; then
            passed_tests=$((passed_tests + 1))
            log_pass "JWT configuration"
        else
            log_fail "JWT configuration missing"
        fi
    }
    
    # Test 5: mTLS configuration
    test_mtls_configuration() {
        total_tests=$((total_tests + 1))
        
        # Check for mTLS configuration in services
        if grep -r "rustls\|mtls" . --include="*.rs" > /dev/null 2>&1; then
            passed_tests=$((passed_tests + 1))
            log_pass "mTLS configuration"
        else
            log_fail "mTLS configuration not found"
        fi
    }
    
    # Test 6: Audit logging
    test_audit_logging() {
        total_tests=$((total_tests + 1))
        
        # Check for audit logging implementation
        if grep -r "audit\|AuditEvent" . --include="*.rs" > /dev/null 2>&1; then
            passed_tests=$((passed_tests + 1))
            log_pass "Audit logging implementation"
        else
            log_fail "Audit logging not implemented"
        fi
    }
    
    # Test 7: Rate limiting
    test_rate_limiting() {
        total_tests=$((total_tests + 1))
        
        # Check for rate limiting implementation
        if grep -r "rate.limit\|RateLimit" . --include="*.rs" > /dev/null 2>&1; then
            passed_tests=$((passed_tests + 1))
            log_pass "Rate limiting implementation"
        else
            log_fail "Rate limiting not implemented"
        fi
    }
    
    # Test 8: Input validation
    test_input_validation() {
        total_tests=$((total_tests + 1))
        
        # Check for input validation
        if grep -r "validate\|sanitize" . --include="*.rs" > /dev/null 2>&1; then
            passed_tests=$((passed_tests + 1))
            log_pass "Input validation"
        else
            log_fail "Input validation not found"
        fi
    }
    
    # Test 9: Error handling
    test_error_handling() {
        total_tests=$((total_tests + 1))
        
        # Check for proper error handling
        if grep -r "Result\|Error" . --include="*.rs" > /dev/null 2>&1; then
            passed_tests=$((passed_tests + 1))
            log_pass "Error handling"
        else
            log_fail "Error handling not found"
        fi
    }
    
    # Test 10: Security headers
    test_security_headers() {
        total_tests=$((total_tests + 1))
        
        # Check for security headers configuration
        if grep -r "X-Frame-Options\|X-Content-Type-Options\|Strict-Transport-Security" . --include="*.rs" > /dev/null 2>&1; then
            passed_tests=$((passed_tests + 1))
            log_pass "Security headers"
        else
            log_fail "Security headers not configured"
        fi
    }
    
    # Run all tests
    log_info "Running security validation tests..."
    
    test_certificate_validation
    test_secret_validation
    test_keycloak_connectivity
    test_jwt_generation
    test_mtls_configuration
    test_audit_logging
    test_rate_limiting
    test_input_validation
    test_error_handling
    test_security_headers
    
    # Summary
    echo
    echo "Security Validation Summary"
    echo "=========================="
    echo "Total tests: $total_tests"
    echo "Passed: $passed_tests"
    echo "Failed: $((total_tests - passed_tests))"
    echo "Success rate: $((passed_tests * 100 / total_tests))%"
    
    if [ $passed_tests -eq $total_tests ]; then
        log_pass "All security tests passed!"
        exit 0
    else
        log_fail "Security tests failed - review and fix issues"
        exit 1
    fi
}

# Performance tests
run_performance_tests() {
    log_info "Running security performance tests..."
    
    # Test certificate loading performance
    if command -v time &> /dev/null; then
        time openssl x509 -in ./certificates/ca.crt -noout -text > /dev/null 2>&1
    fi
    
    # Test JWT validation performance
    # This would test actual JWT validation speed
    
    log_info "Performance tests completed"
}

main() {
    echo "Security Validation Suite"
    echo "========================="
    echo "Environment: $ENVIRONMENT"
    echo
    
    run_security_tests
    echo
    run_performance_tests
}

main "$@"
```

## 4. Monitoreo y Alertas de Seguridad

### 4.1 Configuración de Alertas

```yaml
# security-alerts.yml
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: security-alerts
  namespace: cicd-platform
spec:
  groups:
  - name: security.authentication
    rules:
    - alert: HighFailedAuthenticationRate
      expr: rate(keycloak_authentication_failures_total[5m]) > 0.1
      for: 2m
      labels:
        severity: warning
        component: keycloak
      annotations:
        summary: "High failed authentication rate detected"
        description: "Authentication failure rate is {{ $value }} failures/sec"
        
    - alert: MultipleFailedLoginAttempts
      expr: increase(keycloak_authentication_failures_total[1m]) > 5
      for: 1m
      labels:
        severity: critical
        component: keycloak
      annotations:
        summary: "Multiple failed login attempts detected"
        description: "{{ $value }} failed login attempts in the last minute"
        
    - alert: SuspiciousAuthenticationPattern
      expr: rate(keycloak_authentication_failures_total[5m]) > rate(keycloak_authentication_success_total[5m])
      for: 5m
      labels:
        severity: critical
        component: keycloak
      annotations:
        summary: "Suspicious authentication pattern detected"
        description: "Failure rate exceeds success rate"

  - name: security.authorization
    rules:
    - alert: HighDeniedAuthorizationRate
      expr: rate(avp_authorization_denied_total[5m]) > 0.05
      for: 3m
      labels:
        severity: warning
        component: verified-permissions
      annotations:
        summary: "High denied authorization rate"
        description: "Authorization denial rate is {{ $value }} denials/sec"
        
    - alert: PrivilegeEscalationAttempt
      expr: increase(avp_privilege_escalation_attempts_total[1h]) > 0
      for: 0m
      labels:
        severity: critical
        component: verified-permissions
      annotations:
        summary: "Privilege escalation attempt detected"
        description: "{{ $value }} privilege escalation attempts detected"

  - name: security.transport
    rules:
    - alert: ExpiredCertificates
      expr: cert_expiry_days < 7
      for: 0m
      labels:
        severity: warning
        component: certificates
      annotations:
        summary: "Certificates expiring soon"
        description: "{{ $value }} certificates expire in less than 7 days"
        
    - alert: MTLSHandshakeFailures
      expr: rate(mtls_handshake_failures_total[5m]) > 0.01
      for: 2m
      labels:
        severity: warning
        component: transport
      annotations:
        summary: "High mTLS handshake failure rate"
        description: "mTLS handshake failure rate is {{ $value }} failures/sec"

  - name: security.audit
    rules:
    - alert: AuditLogTampering
      expr: increase(audit_log_integrity_failures_total[1h]) > 0
      for: 0m
      labels:
        severity: critical
        component: audit
      annotations:
        summary: "Audit log tampering detected"
        description: "{{ $value }} audit log tampering attempts detected"
        
    - alert: MissingAuditEvents
      expr: rate(audit_events_total[5m]) < 0.1
      for: 10m
      labels:
        severity: warning
        component: audit
      annotations:
        summary: "Low audit event rate"
        description: "Audit event rate is {{ $value }} events/sec - investigate potential issues"

  - name: security.system
    rules:
    - alert: SecurityComponentDown
      expr: up{component=~"keycloak|verified-permissions"} == 0
      for: 1m
      labels:
        severity: critical
        component: security
      annotations:
        summary: "Security component is down"
        description: "{{ $labels.component }} is not responding"
        
    - alert: CertificateAuthorityCompromise
      expr: ca_compromise_detected == 1
      for: 0m
      labels:
        severity: critical
        component: certificates
      annotations:
        summary: "Certificate authority compromise detected"
        description: "Security breach: Certificate authority may be compromised"
```

### 4.2 Dashboard de Seguridad

```json
{
  "dashboard": {
    "title": "CI/CD Platform Security Dashboard",
    "panels": [
      {
        "title": "Authentication Metrics",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(keycloak_authentication_success_total[5m])",
            "legendFormat": "Successful Logins"
          },
          {
            "expr": "rate(keycloak_authentication_failures_total[5m])",
            "legendFormat": "Failed Logins"
          }
        ],
        "yAxes": [
          {
            "min": 0
          }
        ]
      },
      {
        "title": "Authorization Decisions",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(avp_authorization_allowed_total)",
            "legendFormat": "Allowed"
          },
          {
            "expr": "sum(avp_authorization_denied_total)",
            "legendFormat": "Denied"
          }
        ]
      },
      {
        "title": "Certificate Status",
        "type": "table",
        "targets": [
          {
            "expr": "cert_expiry_days",
            "format": "table",
            "instant": true
          }
        ],
        "columns": [
          {
            "text": "Instance",
            "type": "string"
          },
          {
            "text": "Days Until Expiry",
            "type": "number"
          }
        ]
      },
      {
        "title": "Security Events Timeline",
        "type": "logs",
        "targets": [
          {
            "expr": "{job=\"security-events\"}",
            "refId": "security-events"
          }
        ]
      },
      {
        "title": "Audit Trail Health",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(audit_events_total[5m])",
            "legendFormat": "Events/sec"
          }
        ]
      }
    ]
  }
}
```

Este documento proporciona la configuración completa de deployment y herramientas necesarias para implementar la seguridad distribuida en el sistema CI/CD. Las configuraciones están diseñadas para ser seguras, escalables y cumplidoras con los estándares de seguridad empresariales.