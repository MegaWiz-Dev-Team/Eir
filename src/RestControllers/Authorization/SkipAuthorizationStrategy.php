<?php

namespace OpenEMR\RestControllers\Authorization;

use OpenEMR\Common\Auth\UuidUserAccount;
use OpenEMR\Common\Http\HttpRestRequest;
use OpenEMR\Common\Logging\SystemLoggerAwareTrait;
use OpenEMR\Services\UserService;

class SkipAuthorizationStrategy implements IAuthorizationStrategy
{
    use SystemLoggerAwareTrait;

    /**
     * Environment variable holding the secret shared with eir-gateway.
     * Unset, empty or shorter than GATEWAY_SECRET_MIN_LENGTH means no request is
     * ever trusted as coming from the gateway (fail closed).
     */
    public const GATEWAY_SECRET_ENV = 'EIR_GATEWAY_SECRET';

    /**
     * Request header eir-gateway sends the shared secret in. A bare
     * "X-Gateway: eir-gateway" header is informational only and is never trusted.
     */
    public const GATEWAY_SECRET_HEADER = 'X-Eir-Gateway-Secret';

    public const GATEWAY_SECRET_MIN_LENGTH = 32;

    /**
     * @var string[] List of routes to skip authorization for
     */
    private array $skipRoutes = [];

    private bool $skipOptionsMethod = true;

    private ?UserService $userService = null;

    private ?string $gatewaySecret = null;

    private bool $gatewaySecretLoaded = false;

    /**
     * Override the gateway secret (tests / dependency injection). Passing null
     * disables gateway trust.
     */
    public function setGatewaySecret(?string $gatewaySecret): void
    {
        $this->gatewaySecret = $gatewaySecret;
        $this->gatewaySecretLoaded = true;
    }

    private function getGatewaySecret(): ?string
    {
        if (!$this->gatewaySecretLoaded) {
            $value = getenv(self::GATEWAY_SECRET_ENV);
            $this->gatewaySecret = is_string($value) ? $value : null;
            $this->gatewaySecretLoaded = true;
        }
        return $this->gatewaySecret;
    }

    /**
     * True only when the request carries the secret shared with eir-gateway,
     * compared in constant time. Fails closed when no usable secret is configured.
     */
    public function isTrustedGatewayRequest(HttpRestRequest $request): bool
    {
        $expected = trim($this->getGatewaySecret() ?? '');
        if (strlen($expected) < self::GATEWAY_SECRET_MIN_LENGTH) {
            return false;
        }
        $presented = $request->headers->get(self::GATEWAY_SECRET_HEADER);
        if (!is_string($presented) || $presented === '') {
            return false;
        }
        if (hash_equals($expected, $presented)) {
            return true;
        }
        // never log the presented value
        $this->getSystemLogger()?->warning(
            "SkipAuthorizationStrategy: rejected gateway trust, shared secret mismatch",
            ['clientIp' => $request->getClientIp()]
        );
        return false;
    }

    public function getUserService(): UserService
    {
        if (!isset($this->userService)) {
            $this->userService = new UserService();
        }
        return $this->userService;
    }

    public function setUserService(UserService $userService): void
    {
        $this->userService = $userService;
    }

    public function shouldSkipOptionsMethod(bool $skipOptionsMethod): void
    {
        $this->skipOptionsMethod = $skipOptionsMethod;
    }

    public function addSkipRoute(string $route): void
    {
        $this->skipRoutes[] = $route;
    }

    public function shouldProcessRequest(HttpRestRequest $request): bool
    {
        if ($request->getMethod() === 'OPTIONS' && $this->skipOptionsMethod) {
            return true;
        }

        if ($this->isTrustedGatewayRequest($request)) {
            return true;
        }

        $pathInfo = $request->getPathInfo();
        $sitePath = "/" . $request->getRequestSite();
        if (str_starts_with($pathInfo, $sitePath)) {
            $pathInfo = substr($pathInfo, strlen($sitePath));
        }
        foreach ($this->skipRoutes as $route) {
            if (str_starts_with($route, $pathInfo)) {
                return true;
            }
        }
        return false;
    }

    public function authorizeRequest(HttpRestRequest $request): bool
    {
        // No authorization needed for skipped routes
        $request->attributes->set('skipAuthorization', true);
        // if we have a session we can set the userId and tokenId attributes
        $session = $request->getSession();
        // userId is populated by the bearer token authorization strategy
        $userId = $session->get('authUserId');
        $userRole = UuidUserAccount::USER_ROLE_SYSTEM; // Default to system role if no userId is set
        if (!empty($userId)) {
            // TODO: how do we want to handle patient accounts?  This doesn't accommodate that.
            $userService = $this->getUserService();
            $user = $userService->getUser($userId);
            $userRole = UuidUserAccount::USER_ROLE_USERS;
            // Set the user in the request for HttpRestRequest
            $request->setRequestUser($user['uuid'], $user);
        }
        $request->setRequestUserRole($userRole);
        $request->attributes->set('userId', $userId);
        $request->attributes->set('clientId', null);
        $request->attributes->set('tokenId', null);
        $request->setRequestUserRole($userRole);
        return true;
    }
}
