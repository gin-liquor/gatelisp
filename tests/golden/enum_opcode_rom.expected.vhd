library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_enum_opcode_rom is
  port (
    gl_p0_instruction : in unsigned(23 downto 0);
    gl_p1_opcode : out unsigned(4 downto 0);
    gl_p2_is_jump : out std_logic;
    gl_p3_micro_value : out unsigned(23 downto 0)
  );
end entity gl_m0_enum_opcode_rom;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_enum_opcode_rom is
  constant gl_enum_opcode_jump : unsigned(4 downto 0) := to_unsigned(3, 5);
  constant gl_enum_opcode_jump_if : unsigned(4 downto 0) := to_unsigned(4, 5);
  type gl_rom_microcode_t is array (
    0 to 255
) of unsigned(23 downto 0);
  constant gl_rom_microcode : gl_rom_microcode_t := (
    0 => "000000000000000000000000",
    1 => "000100000000000000000001",
    2 => "001000000000000000000010",
    others => "000000000000000000000000"
  );
  signal gl_s1_opcode : unsigned(4 downto 0);
  signal gl_s2_is_jump : std_logic;
  signal gl_s3_micro_value : unsigned(23 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s1_opcode <= unsigned(gl_p0_instruction(((19 + 5) - 1) downto 19));
  end process gl_comb_0;
  gl_comb_1 : process(all)
    variable gl_tmp_0 : unsigned(4 downto 0);
    variable gl_tmp_1 : std_logic;
  begin
    gl_tmp_0 := gl_s1_opcode;
    if (gl_tmp_0 = gl_enum_opcode_jump) then
      gl_tmp_1 := '1';
    else
      if (gl_tmp_0 = gl_enum_opcode_jump_if) then
        gl_tmp_1 := '1';
      else
        gl_tmp_1 := '0';
      end if;
    end if;
    gl_s2_is_jump <= gl_tmp_1;
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s3_micro_value <= gl_rom_microcode(to_integer(unsigned(gl_p0_instruction(((0 + 8) - 1) downto 0))));
  end process gl_comb_2;
  gl_p1_opcode <= gl_s1_opcode;
  gl_p2_is_jump <= gl_s2_is_jump;
  gl_p3_micro_value <= gl_s3_micro_value;
end architecture rtl;
