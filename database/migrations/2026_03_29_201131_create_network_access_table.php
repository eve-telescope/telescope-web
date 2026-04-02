<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('network_access', function (Blueprint $table) {
            $table->id();
            $table->foreignId('intel_network_id')->constrained('intel_networks')->cascadeOnDelete();
            $table->string('accessible_type');
            $table->unsignedBigInteger('accessible_id');
            $table->string('permission')->default('viewer');
            $table->boolean('is_owner')->default(false);
            $table->timestamp('expires_at')->nullable();
            $table->timestamps();

            $table->index(['accessible_type', 'accessible_id']);
            $table->unique(['intel_network_id', 'accessible_type', 'accessible_id'], 'network_access_unique');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('network_access');
    }
};
