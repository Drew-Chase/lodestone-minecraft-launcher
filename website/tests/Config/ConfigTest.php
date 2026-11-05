<?php

declare(strict_types=1);

namespace tests\Config;

use app\Config\Config;
use PHPUnit\Framework\TestCase;
use RuntimeException;

final class ConfigTest extends TestCase
{
    /** @var list<string> */
    private array $dirs = [];

    /** @var array<string, array{0: string|false, 1: string|null}> Original getenv() and $_ENV values. */
    private array $saved = [];

    protected function setUp(): void
    {
        Config::reset();
    }

    protected function tearDown(): void
    {
        Config::reset();

        foreach ($this->saved as $key => [$env, $super]) {
            if ($env === false) {
                putenv($key);
            } else {
                putenv("$key=$env");
            }
            if ($super === null) {
                unset($_ENV[$key]);
            } else {
                $_ENV[$key] = $super;
            }
        }

        foreach ($this->dirs as $dir) {
            $this->removeDir($dir);
        }
    }

    public function testEnvironmentVariableBeatsDotenv(): void
    {
        $dir = $this->apiDir(['.env' => "LODESTONE_TEST_KEY=from-file\n"]);
        $this->setEnv('LODESTONE_TEST_KEY', 'from-env');

        Config::load($dir);

        self::assertSame('from-env', Config::get('LODESTONE_TEST_KEY'));
    }

    public function testEmptyEnvironmentVariableCountsAsUnset(): void
    {
        $dir = $this->apiDir(['.env' => "LODESTONE_TEST_KEY=from-file\n"]);
        $this->setEnv('LODESTONE_TEST_KEY', '');

        Config::load($dir);

        self::assertSame('from-file', Config::get('LODESTONE_TEST_KEY'));
    }

    public function testProductionFileBeatsDotenv(): void
    {
        $dir = $this->apiDir([
            '.env' => "LODESTONE_TEST_KEY=from-env-file\n",
            'config/production.php' => "<?php\n\nreturn ['LODESTONE_TEST_KEY' => 'from-production'];\n",
        ]);
        $this->setEnv('LODESTONE_TEST_KEY', null);

        Config::load($dir);

        self::assertSame('from-production', Config::get('LODESTONE_TEST_KEY'));
    }

    public function testSetBeatsEveryOtherSource(): void
    {
        $dir = $this->apiDir(['.env' => "LODESTONE_TEST_KEY=from-file\n"]);
        $this->setEnv('LODESTONE_TEST_KEY', 'from-env');

        Config::load($dir);
        Config::set('LODESTONE_TEST_KEY', 'from-set');

        self::assertSame('from-set', Config::get('LODESTONE_TEST_KEY'));
    }

    public function testDotenvParsing(): void
    {
        $dir = $this->apiDir(['.env' => implode("\n", [
            '# a comment line',
            '',
            'export LODESTONE_TEST_EXPORTED="quoted value"',
            "LODESTONE_TEST_SINGLE='single quoted'",
            'LODESTONE_TEST_COMMENT=plain # trailing comment',
            'LODESTONE_TEST_QUOTED_HASH="keep # this"',
            'LODESTONE_TEST_NO_SPACE=a#b',
            'LODESTONE_TEST_EMPTY=',
            '',
        ])]);
        foreach (['LODESTONE_TEST_EXPORTED', 'LODESTONE_TEST_SINGLE', 'LODESTONE_TEST_COMMENT',
            'LODESTONE_TEST_QUOTED_HASH', 'LODESTONE_TEST_NO_SPACE', 'LODESTONE_TEST_EMPTY'] as $key) {
            $this->setEnv($key, null);
        }

        Config::load($dir);

        self::assertSame('quoted value', Config::get('LODESTONE_TEST_EXPORTED'));
        self::assertSame('single quoted', Config::get('LODESTONE_TEST_SINGLE'));
        self::assertSame('plain', Config::get('LODESTONE_TEST_COMMENT'));
        self::assertSame('keep # this', Config::get('LODESTONE_TEST_QUOTED_HASH'));
        self::assertSame('a#b', Config::get('LODESTONE_TEST_NO_SPACE'));
        self::assertSame('fallback', Config::get('LODESTONE_TEST_EMPTY', 'fallback'));
    }

    public function testGetReturnsDefaultWhenUnset(): void
    {
        $dir = $this->apiDir();
        $this->setEnv('LODESTONE_TEST_MISSING', null);

        Config::load($dir);

        self::assertNull(Config::get('LODESTONE_TEST_MISSING'));
        self::assertSame('fallback', Config::get('LODESTONE_TEST_MISSING', 'fallback'));
    }

    public function testNeedThrowsWithTheKeyName(): void
    {
        $dir = $this->apiDir();
        $this->setEnv('LODESTONE_TEST_MISSING', null);

        Config::load($dir);

        $this->expectException(RuntimeException::class);
        $this->expectExceptionMessage('Missing configuration: LODESTONE_TEST_MISSING');

        Config::need('LODESTONE_TEST_MISSING');
    }

    public function testEnvironmentDefaultsToProduction(): void
    {
        $dir = $this->apiDir();
        $this->setEnv('APP_ENV', null);

        Config::load($dir);

        self::assertSame('production', Config::environment());
        self::assertTrue(Config::isProduction());
    }

    public function testEnvironmentReadsAppEnv(): void
    {
        $dir = $this->apiDir(['.env' => "APP_ENV=development\n"]);
        $this->setEnv('APP_ENV', null);

        Config::load($dir);

        self::assertSame('development', Config::environment());
        self::assertFalse(Config::isProduction());
    }

    public function testDevelopmentModeReadsFileBeforeProcess(): void
    {
        $dir = $this->apiDir(['.env' => "APP_ENV=development\nDB_NAME=from_file\n"]);
        $this->setEnv('APP_ENV', null);
        $this->setEnv('DB_NAME', 'from_process');

        Config::load($dir);

        self::assertSame('from_file', Config::get('DB_NAME'));
        self::assertSame('development', Config::environment());
    }

    public function testProcessAppEnvSelectsEnvironmentFirstRule(): void
    {
        $dir = $this->apiDir(['.env' => "APP_ENV=development\nDB_NAME=from_file\n"]);
        $this->setEnv('APP_ENV', 'test');
        $this->setEnv('DB_NAME', 'from_process');

        Config::load($dir);

        self::assertSame('from_process', Config::get('DB_NAME'));
        self::assertSame('test', Config::environment());
    }

    public function testEnvironmentAndPrecedenceUseOneValue(): void
    {
        $dir = $this->apiDir(['.env' => "APP_ENV=production\nDB_NAME=from_file\n"]);
        $this->setEnv('APP_ENV', 'development');
        $this->setEnv('DB_NAME', 'from_process');

        Config::load($dir);

        self::assertSame('development', Config::environment());
        self::assertSame('from_file', Config::get('DB_NAME'));
    }

    public function testSetAppEnvOverridesEnvironment(): void
    {
        $dir = $this->apiDir();
        $this->setEnv('APP_ENV', null);

        Config::load($dir);
        Config::set('APP_ENV', 'test');

        self::assertSame('test', Config::environment());
    }

    /**
     * @param array<string, string> $files Relative path to content.
     */
    private function apiDir(array $files = []): string
    {
        $dir = sys_get_temp_dir() . DIRECTORY_SEPARATOR . 'lodestone-config-' . bin2hex(random_bytes(6));
        mkdir($dir, 0777, true);
        $this->dirs[] = $dir;

        foreach ($files as $name => $content) {
            $path = $dir . DIRECTORY_SEPARATOR . str_replace('/', DIRECTORY_SEPARATOR, $name);
            if (!is_dir(dirname($path))) {
                mkdir(dirname($path), 0777, true);
            }
            file_put_contents($path, $content);
        }

        return $dir;
    }

    private function setEnv(string $key, ?string $value): void
    {
        if (!array_key_exists($key, $this->saved)) {
            $this->saved[$key] = [getenv($key), $_ENV[$key] ?? null];
        }

        if ($value === null) {
            putenv($key);
            unset($_ENV[$key]);
        } else {
            putenv("$key=$value");
            $_ENV[$key] = $value;
        }
    }

    private function removeDir(string $dir): void
    {
        $items = new \RecursiveIteratorIterator(
            new \RecursiveDirectoryIterator($dir, \FilesystemIterator::SKIP_DOTS),
            \RecursiveIteratorIterator::CHILD_FIRST
        );
        foreach ($items as $item) {
            $item->isDir() ? rmdir($item->getPathname()) : unlink($item->getPathname());
        }
        rmdir($dir);
    }
}
