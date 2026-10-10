<?php

namespace OpenEMR\Tests\RestControllers\Authorization;

use OpenEMR\Common\Auth\UuidUserAccount;
use OpenEMR\Common\Http\HttpRestRequest;
use OpenEMR\Common\Logging\SystemLogger;
use OpenEMR\RestControllers\Authorization\SkipAuthorizationStrategy;
use PHPUnit\Framework\TestCase;
use Symfony\Component\HttpFoundation\Session\Session;
use Symfony\Component\HttpFoundation\Session\SessionInterface;
use Symfony\Component\HttpFoundation\Session\Storage\MockFileSessionStorageFactory;

class SkipAuthorizationStrategyTest extends TestCase
{
    public function testShouldSkipOptionsMethod(): void
    {
        $request = HttpRestRequest::create("/apis/default/fhir/Patient", "OPTIONS");
        $skipAuthorizationStrategy = new SkipAuthorizationStrategy();
        $skipAuthorizationStrategy->shouldSkipOptionsMethod(false);
        // no routes so first we should skip
        $this->assertFalse($skipAuthorizationStrategy->shouldProcessRequest($request), "Options should NOT be skipped");
        $skipAuthorizationStrategy->shouldSkipOptionsMethod(true);
        $this->assertTrue($skipAuthorizationStrategy->shouldProcessRequest($request), "Options should be skipped when flag is set");
    }

    private const TEST_GATEWAY_SECRET = 'test-only-gateway-secret-xxxxxxxxxxxxxxxx';

    /**
     * @param array<string, string> $headers
     */
    private function gatewayRequest(array $headers): HttpRestRequest
    {
        $server = [];
        foreach ($headers as $name => $value) {
            $server['HTTP_' . strtoupper(str_replace('-', '_', $name))] = $value;
        }
        return HttpRestRequest::create("/apis/default/fhir/Patient", "GET", [], [], [], $server);
    }

    public function testGatewayHeaderAloneIsNeverTrusted(): void
    {
        $strategy = new SkipAuthorizationStrategy();
        $strategy->setGatewaySecret(null);
        $request = $this->gatewayRequest(['X-Gateway' => 'eir-gateway']);
        $this->assertFalse($strategy->shouldProcessRequest($request), "X-Gateway header alone must not skip authorization");

        $strategy->setGatewaySecret(self::TEST_GATEWAY_SECRET);
        $this->assertFalse($strategy->shouldProcessRequest($request), "X-Gateway header without the shared secret must not skip authorization");
    }

    public function testValidGatewaySecretIsTrusted(): void
    {
        $strategy = new SkipAuthorizationStrategy();
        $strategy->setGatewaySecret(self::TEST_GATEWAY_SECRET);
        $request = $this->gatewayRequest([SkipAuthorizationStrategy::GATEWAY_SECRET_HEADER => self::TEST_GATEWAY_SECRET]);
        $this->assertTrue($strategy->shouldProcessRequest($request), "Request carrying the shared gateway secret should be trusted");
    }

    public function testConfiguredSecretIsTrimmed(): void
    {
        $strategy = new SkipAuthorizationStrategy();
        $strategy->setGatewaySecret(self::TEST_GATEWAY_SECRET . "\n");
        $request = $this->gatewayRequest([SkipAuthorizationStrategy::GATEWAY_SECRET_HEADER => self::TEST_GATEWAY_SECRET]);
        $this->assertTrue($strategy->shouldProcessRequest($request), "A trailing newline in the configured secret should not break gateway trust");
    }

    public function testWrongGatewaySecretIsRejectedAndLoggedWithoutTheValue(): void
    {
        $strategy = new SkipAuthorizationStrategy();
        $strategy->setGatewaySecret(self::TEST_GATEWAY_SECRET);
        $presented = 'wrong-secret-yyyyyyyyyyyyyyyyyyyyyyyyyyyy';
        $logger = $this->createMock(SystemLogger::class);
        $logger->expects($this->once())
            ->method('warning')
            ->with(
                $this->stringContains('shared secret mismatch'),
                $this->callback(fn(array $context): bool => !str_contains(json_encode($context, JSON_THROW_ON_ERROR), $presented)
                    && !str_contains(json_encode($context, JSON_THROW_ON_ERROR), self::TEST_GATEWAY_SECRET))
            );
        $strategy->setSystemLogger($logger);
        $request = $this->gatewayRequest([
            'X-Gateway' => 'eir-gateway',
            SkipAuthorizationStrategy::GATEWAY_SECRET_HEADER => $presented,
        ]);
        $this->assertFalse($strategy->shouldProcessRequest($request), "Wrong gateway secret must not skip authorization");
    }

    public function testMissingOrShortSecretFailsClosed(): void
    {
        foreach ([null, '', '   ', 'short-secret'] as $configured) {
            $strategy = new SkipAuthorizationStrategy();
            $strategy->setGatewaySecret($configured);
            $request = $this->gatewayRequest([SkipAuthorizationStrategy::GATEWAY_SECRET_HEADER => (string)$configured]);
            $this->assertFalse(
                $strategy->shouldProcessRequest($request),
                "Gateway trust must fail closed when the configured secret is " . var_export($configured, true)
            );
        }
    }

    public function testGatewaySecretIsReadFromEnvironment(): void
    {
        $previous = getenv(SkipAuthorizationStrategy::GATEWAY_SECRET_ENV);
        try {
            putenv(SkipAuthorizationStrategy::GATEWAY_SECRET_ENV . '=' . self::TEST_GATEWAY_SECRET);
            $request = $this->gatewayRequest([SkipAuthorizationStrategy::GATEWAY_SECRET_HEADER => self::TEST_GATEWAY_SECRET]);
            $this->assertTrue((new SkipAuthorizationStrategy())->shouldProcessRequest($request), "Secret from the environment should be honoured");

            putenv(SkipAuthorizationStrategy::GATEWAY_SECRET_ENV);
            $this->assertFalse((new SkipAuthorizationStrategy())->shouldProcessRequest($request), "Unset environment secret must fail closed");
        } finally {
            putenv($previous === false
                ? SkipAuthorizationStrategy::GATEWAY_SECRET_ENV
                : SkipAuthorizationStrategy::GATEWAY_SECRET_ENV . '=' . $previous);
        }
    }

    public function testAddSkipRoute(): void
    {
        $this->markTestIncomplete("Test is incomplete");
    }

    public function testShouldProcessRequest(): void
    {
        $this->markTestIncomplete("Test is incomplete");
    }

    public function testAuthorizeRequestWithValidSkippedPath(): void
    {
        $userId = 1;
        $request = HttpRestRequest::create("/apis/default/fhir/metadata", "GET");
        $session = $this->getMockSessionForRequest($request);
        $session->set("authUserId", $userId);
        $request->setSession($session);
        $skipAuthorizationStrategy = new SkipAuthorizationStrategy();
        $mockUserService = $this->createMock(\OpenEMR\Services\UserService::class);
        $userUuid = '123e4567-e89b-12d3-a456-426614174000';
        $mockUserService->expects($this->once())
            ->method('getUser')
            ->with($userId)
            ->willReturn(['id' => $userId, 'uuid' => $userUuid, 'username' => 'testuser']);
        $skipAuthorizationStrategy->setUserService($mockUserService);
        $skipAuthorizationStrategy->addSkipRoute("/apis/default/fhir/metadata");
        $this->assertTrue($skipAuthorizationStrategy->authorizeRequest($request), "Path should be authorized for skipped path");
        // now assert all the attributes that were populated.

        $this->assertEquals($userId, $request->getRequestUser()['id'], "Expected user ID to be set to 1 for skipped path");
        $this->assertEquals($userUuid, $request->getRequestUser()['uuid'], "Expected user uuid to be set for skipped path");
        $this->assertEquals(null, $request->attributes->get('clientId'), "Expected clientId to be null for skipped path");
        $this->assertEquals(null, $request->attributes->get('tokenId'), "Expected tokenId to be null for skipped path");
        $this->assertEquals(UuidUserAccount::USER_ROLE_USERS, $request->getRequestUserRole(), "Expected user role to be 'users' for skipped path");
        $this->assertEquals($userId, $request->attributes->get('userId'), "Expected userId attribute to be set to 1 for skipped path");
    }

    private function getMockSessionForRequest(HttpRestRequest $request): SessionInterface
    {
        $sessionFactory = new MockFileSessionStorageFactory();
        $sessionStorage = $sessionFactory->createStorage($request);
        $session = new Session($sessionStorage);
        $session->start();
        $session->set("site_id", "default");
        return $session;
    }
}
