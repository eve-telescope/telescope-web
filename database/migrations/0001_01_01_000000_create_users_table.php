<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('alliances', function (Blueprint $table) {
            $table->unsignedBigInteger('id')->primary();
            $table->string('name');
            $table->string('ticker');
            $table->timestamp('last_updated')->nullable();
            $table->timestamps();
        });

        Schema::create('corporations', function (Blueprint $table) {
            $table->unsignedBigInteger('id')->primary();
            $table->string('name');
            $table->string('ticker');
            $table->unsignedBigInteger('alliance_id')->nullable();
            $table->unsignedInteger('member_count')->nullable();
            $table->timestamp('last_updated')->nullable();
            $table->timestamps();

            $table->foreign('alliance_id')->references('id')->on('alliances')->nullOnDelete();
        });

        Schema::create('users', function (Blueprint $table) {
            $table->id();
            $table->unsignedBigInteger('character_id')->unique();
            $table->string('character_name');
            $table->string('character_owner_hash');
            $table->unsignedBigInteger('corporation_id')->nullable();
            $table->unsignedBigInteger('alliance_id')->nullable();
            $table->rememberToken();
            $table->timestamp('last_active_at')->nullable();
            $table->timestamps();

            $table->foreign('corporation_id')->references('id')->on('corporations')->nullOnDelete();
            $table->foreign('alliance_id')->references('id')->on('alliances')->nullOnDelete();
        });

        Schema::create('sessions', function (Blueprint $table) {
            $table->string('id')->primary();
            $table->foreignId('user_id')->nullable()->index();
            $table->string('ip_address', 45)->nullable();
            $table->text('user_agent')->nullable();
            $table->longText('payload');
            $table->integer('last_activity')->index();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('sessions');
        Schema::dropIfExists('users');
        Schema::dropIfExists('corporations');
        Schema::dropIfExists('alliances');
    }
};
