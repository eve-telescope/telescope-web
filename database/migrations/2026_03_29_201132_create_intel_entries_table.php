<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('intel_entries', function (Blueprint $table) {
            $table->id();
            $table->foreignId('intel_network_id')->constrained('intel_networks')->cascadeOnDelete();
            $table->string('entity_type');
            $table->unsignedBigInteger('entity_id');
            $table->string('entity_name');
            $table->string('color', 7);
            $table->string('label');
            $table->foreignId('added_by')->constrained('users')->cascadeOnDelete();
            $table->timestamps();

            $table->unique(['intel_network_id', 'entity_type', 'entity_id'], 'intel_entries_unique');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('intel_entries');
    }
};
